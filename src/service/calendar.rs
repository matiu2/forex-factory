use chrono::{Local, NaiveDate};
use color_eyre::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::scraper::{CalendarParser, HttpCalendarFetcher};
use crate::types::{EconomicEvent, EventQuery, Impact};

/// High-level service for fetching and querying economic events.
pub struct CalendarService {
    fetcher: Arc<Mutex<HttpCalendarFetcher>>,
    parser: CalendarParser,
}

impl CalendarService {
    /// Create a new calendar service.
    pub fn new() -> Result<Self> {
        let fetcher = HttpCalendarFetcher::new()?;
        let parser = CalendarParser::new()?;

        Ok(Self {
            fetcher: Arc::new(Mutex::new(fetcher)),
            parser,
        })
    }

    /// Query events matching the given criteria.
    pub async fn query_events(&self, query: &EventQuery) -> Result<Vec<EconomicEvent>> {
        // Determine which date to fetch
        let base_date = query.from_date.unwrap_or_else(|| Local::now().date_naive());

        info!("Fetching calendar for date: {base_date}");

        // Fetch HTML from Forex Factory
        let fetcher = self.fetcher.lock().await;
        let html = fetcher.fetch_date(base_date).await?;
        drop(fetcher); // Release lock early

        // Parse events
        let events = self.parser.parse(&html, base_date)?;

        // Filter events based on query
        let min_impact = query.min_impact.unwrap_or(Impact::Low);
        let filtered: Vec<EconomicEvent> = events
            .into_iter()
            .filter(|e| e.meets_impact(min_impact))
            .filter(|e| e.matches_currencies(&query.currencies))
            .filter(|e| query.datetime_in_range(&e.datetime))
            .collect();

        info!("Found {} events matching query", filtered.len());
        Ok(filtered)
    }

    /// Get events for today.
    pub async fn get_today_events(&self) -> Result<Vec<EconomicEvent>> {
        let today = Local::now().date_naive();
        let fetcher = self.fetcher.lock().await;
        let html = fetcher.fetch_today().await?;
        drop(fetcher);

        self.parser.parse(&html, today)
    }

    /// Get events for this week.
    pub async fn get_week_events(&self) -> Result<Vec<EconomicEvent>> {
        let today = Local::now().date_naive();
        let fetcher = self.fetcher.lock().await;
        let html = fetcher.fetch_this_week().await?;
        drop(fetcher);

        self.parser.parse(&html, today)
    }

    /// Get events for a specific week containing the given date.
    pub async fn get_week_events_for(&self, date: NaiveDate) -> Result<Vec<EconomicEvent>> {
        let fetcher = self.fetcher.lock().await;
        let html = fetcher.fetch_date(date).await?;
        drop(fetcher);

        self.parser.parse(&html, date)
    }
}
