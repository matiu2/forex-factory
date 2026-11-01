use chrono::{Local, NaiveDate};
use tracing::info;

use crate::error::Result;
use crate::scraper::{CalendarParser, HttpCalendarFetcher};
use crate::types::{EconomicEvent, EventQuery, Impact};

/// High-level service for fetching and querying economic events.
pub struct CalendarService {
    fetcher: HttpCalendarFetcher,
    parser: CalendarParser,
}

impl CalendarService {
    /// Create a new calendar service.
    ///
    /// On native targets, automatically detects the system timezone.
    /// On WASM targets, falls back to UTC unless you use `with_timezone()`.
    pub fn new() -> Result<Self> {
        let fetcher = HttpCalendarFetcher::new()?;
        let parser = CalendarParser::new()?;

        Ok(Self { fetcher, parser })
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

        Ok(Self { fetcher, parser })
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

        // Fetch events for all weeks in range
        let mut all_events = Vec::new();
        let mut current_date = start_date;

        while current_date <= end_date {
            let html = self.fetcher.fetch_date(current_date).await?;
            let events = self.parser.parse_checked(&html, current_date)?;
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
        let html = self.fetcher.fetch_today().await?;
        self.parser.parse_checked(&html, today)
    }

    /// Get events for this week.
    pub async fn get_week_events(&self) -> Result<Vec<EconomicEvent>> {
        let today = Local::now().date_naive();
        let html = self.fetcher.fetch_this_week().await?;
        self.parser.parse_checked(&html, today)
    }

    /// Fetch a week as strict high-impact entry-rule evidence. Display/query
    /// parsing remains available through the ordinary event methods.
    pub async fn get_week_high_impact_evidence_for(
        &self,
        date: NaiveDate,
    ) -> Result<Vec<EconomicEvent>> {
        let html = self.fetcher.fetch_date(date).await?;
        self.parser.parse_high_impact_evidence(&html, date)
    }

    /// Get events for a specific week containing the given date.
    pub async fn get_week_events_for(&self, date: NaiveDate) -> Result<Vec<EconomicEvent>> {
        let html = self.fetcher.fetch_date(date).await?;
        self.parser.parse_checked(&html, date)
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
