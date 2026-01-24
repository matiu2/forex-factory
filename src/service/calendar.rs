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
        let parser = CalendarParser::new()?;

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
    pub async fn query_events(&self, query: &EventQuery) -> Result<Vec<EconomicEvent>> {
        let (from_date, to_date) = query.fetch_date_range();
        let start_date = from_date.unwrap_or_else(|| Local::now().date_naive());
        let end_date = to_date.unwrap_or(start_date);

        info!("EventQuery: {query:?}");

        // Fetch events for all weeks in range
        let mut all_events = Vec::new();
        let mut current_date = start_date;

        while current_date <= end_date {
            let html = self.fetcher.fetch_date(current_date).await?;
            let events = self.parser.parse(&html, current_date);
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
        Ok(self.parser.parse(&html, today))
    }

    /// Get events for this week.
    pub async fn get_week_events(&self) -> Result<Vec<EconomicEvent>> {
        let today = Local::now().date_naive();
        let html = self.fetcher.fetch_this_week().await?;
        Ok(self.parser.parse(&html, today))
    }

    /// Get events for a specific week containing the given date.
    pub async fn get_week_events_for(&self, date: NaiveDate) -> Result<Vec<EconomicEvent>> {
        let html = self.fetcher.fetch_date(date).await?;
        Ok(self.parser.parse(&html, date))
    }
}
