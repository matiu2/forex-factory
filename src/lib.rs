//! Forex Factory Calendar Scraper Library
//!
//! This library provides functionality to scrape economic event data from the
//! Forex Factory calendar website.
//!
//! # Example
//!
//! ```ignore
//! use forex_factory::{CalendarService, EventQuery, Impact, Result};
//!
//! async fn fetch_events() -> Result<()> {
//!     let service = CalendarService::new()?;
//!
//!     // Get all events for this week
//!     let events = service.get_week_events().await?;
//!
//!     // Query with filters
//!     let query = EventQuery::new()
//!         .with_currency_pair("EUR/USD")
//!         .with_min_impact(Impact::Medium);
//!     let filtered = service.query_events(&query).await?;
//!
//!     Ok(())
//! }
//! ```

mod error;
mod scraper;
mod service;
mod types;

// Re-export public types
pub use error::{Error, Result};
pub use scraper::{CalendarParser, HttpCalendarFetcher};
pub use service::CalendarService;
pub use types::{Currency, EconomicEvent, EventQuery, Impact, resolve_currency};
