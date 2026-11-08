use chrono::{Local, NaiveDate};
use tracing::{info, warn};

use crate::error::Result;
use crate::scraper::{CalendarParser, FeedWeek, HttpCalendarFetcher};
use crate::types::{EconomicEvent, EventQuery, EventSource, Impact};

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
        if let Some(events) = self.feed_events_for(today).await {
            let todays = events
                .into_iter()
                .filter(|e| e.datetime.date_naive() == today);
            return Ok(todays.collect());
        }
        let html = self.fetcher.fetch_today().await?;
        self.parser.parse_checked(&html, today)
    }

    /// Get events for this week.
    pub async fn get_week_events(&self) -> Result<Vec<EconomicEvent>> {
        let today = Local::now().date_naive();
        if let Some(events) = self.feed_events_for(today).await {
            return Ok(events);
        }
        let html = self.fetcher.fetch_this_week().await?;
        self.parser.parse_checked(&html, today)
    }

    /// Fetch a week as strict high-impact entry-rule evidence. Display/query
    /// parsing remains available through the ordinary event methods.
    ///
    /// With [`EventSource::PreferThe5ers`] the export answers when it covers
    /// the week. It meets the strict bar by construction: every export event
    /// carries an exact timestamp, "All Day" ones included — which is also
    /// what lets a week with a high-impact "All Day" event be evidence at
    /// all, since the scraper must reject such a week.
    pub async fn get_week_high_impact_evidence_for(
        &self,
        date: NaiveDate,
    ) -> Result<Vec<EconomicEvent>> {
        if let Some(events) = self.feed_events_for(date).await {
            return Ok(events);
        }
        let html = self.fetcher.fetch_date(date).await?;
        self.parser.parse_high_impact_evidence(&html, date)
    }

    /// Get events for a specific week containing the given date.
    pub async fn get_week_events_for(&self, date: NaiveDate) -> Result<Vec<EconomicEvent>> {
        let feed = self.load_feed().await;
        self.week_events(feed.as_ref(), date).await
    }

    /// The events of `date`'s week: from `feed` when it covers that week,
    /// scraped otherwise.
    async fn week_events(
        &self,
        feed: Option<&FeedWeek>,
        date: NaiveDate,
    ) -> Result<Vec<EconomicEvent>> {
        if let Some(events) = covered(feed, date) {
            return Ok(events);
        }
        let html = self.fetcher.fetch_date(date).await?;
        self.parser.parse_checked(&html, date)
    }

    /// The export's events when the source wants them and the export covers
    /// `date`'s week; `None` means scrape.
    async fn feed_events_for(&self, date: NaiveDate) -> Option<Vec<EconomicEvent>> {
        covered(self.load_feed().await.as_ref(), date)
    }

    /// The weekly export, when the source wants it and it can be had.
    ///
    /// Never an error: an unreachable, rate-limited or empty export just
    /// means falling back to scraping, which is still Forex Factory's data.
    async fn load_feed(&self) -> Option<FeedWeek> {
        if self.source != EventSource::PreferThe5ers {
            return None;
        }
        let feed = self.fetcher.fetch_feed().await;
        match feed.and_then(|json| FeedWeek::parse(&json)) {
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

/// `feed`'s events if it covers `date`'s week. Logged when it does not, as
/// the caller then falls back to scraping.
fn covered(feed: Option<&FeedWeek>, date: NaiveDate) -> Option<Vec<EconomicEvent>> {
    let feed = feed?;
    if !feed.covers(date) {
        info!("The5ers calendar export does not cover the week of {date}; scraping");
        return None;
    }
    Some(feed.events().to_vec())
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
