use chrono::{Datelike, Local, NaiveDate};
use tracing::{info, warn};

use crate::error::Result;
use crate::scraper::{CalendarParser, HttpCalendarFetcher, The5ersWeek};
use crate::types::{EconomicEvent, EventQuery, EventSource, Impact};
mod export;

/// High-level service for fetching and querying economic events.
pub struct CalendarService {
    fetcher: HttpCalendarFetcher,
    parser: CalendarParser,
    source: EventSource,
}

impl CalendarService {
    /// Create a new calendar service.
    ///
    /// On native targets, automatically detects the system timezone.
    /// On WASM targets, falls back to UTC unless you use `with_timezone()`.
    pub fn new() -> Result<Self> {
        let fetcher = HttpCalendarFetcher::new()?;
        let parser = CalendarParser::new()?;
        let source = EventSource::default();

        Ok(Self {
            fetcher,
            parser,
            source,
        })
    }

    /// Create a new calendar service with a specific IANA timezone.
    ///
    /// Use this on WASM to set the timezone (get it from JavaScript via
    /// `Intl.DateTimeFormat().resolvedOptions().timeZone`).
    ///
    /// # Example
    /// ```ignore
    /// let service = CalendarService::with_timezone("America/New_York")?;
    /// ```
    pub fn with_timezone(timezone: &str) -> Result<Self> {
        let fetcher = HttpCalendarFetcher::with_timezone(timezone)?;
        let parser = CalendarParser::with_timezone(timezone)?;
        let source = EventSource::default();

        Ok(Self {
            fetcher,
            parser,
            source,
        })
    }

    /// Choose where events come from — see [`EventSource`]. Every fetching
    /// method honours it.
    ///
    /// # Example
    /// ```ignore
    /// let service = CalendarService::new()?.with_source(EventSource::PreferThe5ers);
    /// ```
    pub fn with_source(mut self, source: EventSource) -> Self {
        self.source = source;
        self
    }

    /// Where this service gets its events from.
    pub fn source(&self) -> EventSource {
        self.source
    }

    /// Query events matching the given criteria.
    ///
    /// This method fetches events and filters them based on the query parameters:
    /// - Date/datetime range
    /// - Currency filter
    /// - Minimum impact level
    ///
    /// For queries spanning multiple weeks, it fetches all necessary weeks.
    pub async fn query_events(&self, query: impl AsRef<EventQuery>) -> Result<Vec<EconomicEvent>> {
        let query = query.as_ref();
        let (from_date, to_date) = query.fetch_date_range();
        let start_date = from_date.unwrap_or_else(|| Local::now().date_naive());
        let end_date = to_date.unwrap_or(start_date);

        info!("EventQuery: {query:?}");

        // Fetch events for all weeks in range; the export, if wanted, once.
        let feed = self.load_feed().await;
        let mut all_events = Vec::new();
        let mut current_date = start_date;

        while current_date <= end_date {
            let events = self.week_events(feed.as_ref(), current_date).await?;
            all_events.extend(events);

            // Move to next week (7 days forward)
            current_date += chrono::Duration::days(7);
        }

        // Filter events based on query
        let min_impact = query.min_impact.unwrap_or(Impact::Low);
        let filtered: Vec<EconomicEvent> = all_events
            .into_iter()
            .filter(|e| e.meets_impact(min_impact))
            .filter(|e| e.matches_currency_list(&query.currencies))
            .filter(|e| query.datetime_in_range(&e.datetime))
            .collect();

        info!("Found {} events matching query", filtered.len());
        Ok(filtered)
    }

    /// Get events for today.
    pub async fn get_today_events(&self) -> Result<Vec<EconomicEvent>> {
        let today = Local::now().date_naive();
        if let Some(events) = self.feed_events_for(today, 1).await {
            return Ok(events);
        }
        let html = self.fetcher.fetch_today().await?;
        self.parser.parse_checked(&html, today)
    }

    /// Get events for this week.
    pub async fn get_week_events(&self) -> Result<Vec<EconomicEvent>> {
        let today = Local::now().date_naive();
        // `?week=this` is Forex Factory's own Sunday-to-Saturday week.
        let sunday = today - chrono::Days::new(today.weekday().num_days_from_sunday().into());
        if let Some(events) = self.feed_events_for(sunday, 7).await {
            return Ok(events);
        }
        let html = self.fetcher.fetch_this_week().await?;
        self.parser.parse_checked(&html, today)
    }

    /// Fetch a week as strict high-impact entry-rule evidence. Display/query
    /// parsing remains available through the ordinary event methods.
    ///
    /// The week is the seven days starting at `date` — what Forex Factory's
    /// `?week=<date>` page shows. With [`EventSource::PreferThe5ers`] the
    /// export answers only when it covers all seven days, which a page
    /// rarely lines up with (the export runs Sunday to Sunday, New York
    /// time); use [`get_high_impact_evidence_between`](Self::get_high_impact_evidence_between)
    /// for a window inside the export's week. Every export event carries an exact timestamp, "All Day"
    /// ones included — the time The5ers publish — while the scraper marks
    /// those [`EventTiming::Untimed`](crate::EventTiming), with no instant.
    pub async fn get_week_high_impact_evidence_for(
        &self,
        date: NaiveDate,
    ) -> Result<Vec<EconomicEvent>> {
        if let Some(events) = self.feed_events_for(date, 7).await {
            return Ok(events);
        }
        self.scrape_page(date, true).await
    }

    /// Get the events of the seven days starting at `date` (Forex Factory's
    /// `?week=<date>` page).
    pub async fn get_week_events_for(&self, date: NaiveDate) -> Result<Vec<EconomicEvent>> {
        let feed = self.load_feed().await;
        self.week_events(feed.as_ref(), date).await
    }

    /// The events of the page starting at `date`: from `feed` when it covers
    /// all seven days, scraped otherwise.
    async fn week_events(
        &self,
        feed: Option<&The5ersWeek>,
        date: NaiveDate,
    ) -> Result<Vec<EconomicEvent>> {
        if let Some(events) = self.covered_days(feed, date, 7)? {
            return Ok(events);
        }
        self.scrape_page(date, false).await
    }

    /// The export's events for `days` days from `date` when the source wants
    /// the export and it covers them all; `None` means scrape.
    async fn feed_events_for(&self, date: NaiveDate, days: u64) -> Option<Vec<EconomicEvent>> {
        let feed = self.load_feed().await;
        self.covered_days(feed.as_ref(), date, days)
            .unwrap_or_else(|error| {
                warn!("cannot place {date} in the calendar timezone ({error}); scraping");
                None
            })
    }

    /// The weekly export, when the source wants it and it can be had.
    ///
    /// Never an error: an unreachable, rate-limited or empty export just
    /// means falling back to scraping, which is still Forex Factory's data.
    async fn load_feed(&self) -> Option<The5ersWeek> {
        if self.source != EventSource::PreferThe5ers {
            return None;
        }
        match self.the5ers_week().await {
            Ok(feed) => feed,
            Err(error) => {
                warn!("The5ers calendar export unavailable, falling back to scraping: {error}");
                None
            }
        }
    }
}

#[cfg(test)]
mod timezone_tests {
    use super::*;
    use tracing_subscriber::prelude::*;
    #[test]
    fn service_requests_and_parses_the_same_timezone() {
        tracing_subscriber::registry()
            .with(tracing_error::ErrorLayer::default())
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(tracing_subscriber::fmt::layer().with_test_writer())
            .try_init()
            .ok();
        let service = CalendarService::with_timezone("UTC").unwrap();
        let events = service
            .parser
            .parse_checked(
                include_str!("../../tests/fixtures/utc-release.html"),
                NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
            )
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].datetime.with_timezone(&chrono::Utc).to_rfc3339(),
            "2026-11-06T13:30:00+00:00"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_defaults_to_scraping_and_can_be_chosen() {
        let service = CalendarService::new().unwrap();
        assert_eq!(service.source(), EventSource::ForexFactory);

        let service = service.with_source(EventSource::PreferThe5ers);
        assert_eq!(service.source(), EventSource::PreferThe5ers);
    }
}
