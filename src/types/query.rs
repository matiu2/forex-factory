use chrono::{DateTime, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};

use super::Impact;

/// Query parameters for filtering economic events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventQuery {
    /// Filter by currencies (empty means all currencies)
    /// For currency pairs like "AUD/CHF", both currencies are included
    #[serde(default)]
    pub currencies: Vec<String>,

    /// Start date for the query range
    pub from_date: Option<NaiveDate>,

    /// End date for the query range
    pub to_date: Option<NaiveDate>,

    /// Minimum impact level (defaults to Low if not specified)
    pub min_impact: Option<Impact>,
}

impl EventQuery {
    /// Create a new empty query (matches all events)
    pub fn new() -> Self {
        Self::default()
    }

    /// Set currencies from a currency pair string (e.g., "AUD/CHF")
    /// Extracts both currencies from the pair
    pub fn with_currency_pair(mut self, pair: &str) -> Self {
        let currencies: Vec<String> = pair
            .split(['/', '-', '_'])
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
        self.currencies = currencies;
        self
    }

    /// Set a single currency
    #[allow(dead_code)]
    pub fn with_currency(mut self, currency: &str) -> Self {
        self.currencies = vec![currency.to_uppercase()];
        self
    }

    /// Set multiple currencies
    pub fn with_currencies(mut self, currencies: Vec<String>) -> Self {
        self.currencies = currencies.into_iter().map(|c| c.to_uppercase()).collect();
        self
    }

    /// Set the date range
    pub fn with_date_range(mut self, from: NaiveDate, to: NaiveDate) -> Self {
        self.from_date = Some(from);
        self.to_date = Some(to);
        self
    }

    /// Set the week around a specific date (3 days before and after)
    pub fn with_week_around(mut self, date: NaiveDate) -> Self {
        self.from_date = date.checked_sub_signed(Duration::days(3));
        self.to_date = date.checked_add_signed(Duration::days(3));
        self
    }

    /// Set minimum impact level
    pub fn with_min_impact(mut self, impact: Impact) -> Self {
        self.min_impact = Some(impact);
        self
    }

    /// Set minimum impact level by star rating (1-3)
    pub fn with_min_stars(mut self, stars: u8) -> Self {
        self.min_impact = Impact::from_stars(stars);
        self
    }

    /// Check if an event's datetime falls within the query date range
    pub fn datetime_in_range(&self, datetime: &DateTime<Local>) -> bool {
        let date = datetime.date_naive();

        if let Some(from) = self.from_date
            && date < from
        {
            return false;
        }

        if let Some(to) = self.to_date
            && date > to
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_with_currency_pair() {
        let query = EventQuery::new().with_currency_pair("AUD/CHF");
        assert_eq!(query.currencies, vec!["AUD", "CHF"]);

        let query = EventQuery::new().with_currency_pair("EUR-USD");
        assert_eq!(query.currencies, vec!["EUR", "USD"]);

        let query = EventQuery::new().with_currency_pair("gbp_jpy");
        assert_eq!(query.currencies, vec!["GBP", "JPY"]);
    }

    #[test]
    fn test_with_single_currency() {
        let query = EventQuery::new().with_currency("usd");
        assert_eq!(query.currencies, vec!["USD"]);
    }

    #[test]
    fn test_with_week_around() {
        let date = NaiveDate::from_ymd_opt(2025, 6, 4).unwrap();
        let query = EventQuery::new().with_week_around(date);

        assert_eq!(
            query.from_date,
            Some(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap())
        );
        assert_eq!(
            query.to_date,
            Some(NaiveDate::from_ymd_opt(2025, 6, 7).unwrap())
        );
    }

    #[test]
    fn test_with_min_stars() {
        let query = EventQuery::new().with_min_stars(2);
        assert_eq!(query.min_impact, Some(Impact::Medium));
    }

    #[test]
    fn test_datetime_in_range() {
        let query = EventQuery::new().with_date_range(
            NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 6, 7).unwrap(),
        );

        // Within range
        let dt = Local.with_ymd_and_hms(2025, 6, 4, 12, 0, 0).unwrap();
        assert!(query.datetime_in_range(&dt));

        // Before range
        let dt = Local.with_ymd_and_hms(2025, 5, 31, 12, 0, 0).unwrap();
        assert!(!query.datetime_in_range(&dt));

        // After range
        let dt = Local.with_ymd_and_hms(2025, 6, 8, 12, 0, 0).unwrap();
        assert!(!query.datetime_in_range(&dt));

        // Boundary dates (inclusive)
        let dt = Local.with_ymd_and_hms(2025, 6, 1, 0, 0, 0).unwrap();
        assert!(query.datetime_in_range(&dt));

        let dt = Local.with_ymd_and_hms(2025, 6, 7, 23, 59, 59).unwrap();
        assert!(query.datetime_in_range(&dt));
    }

    #[test]
    fn test_datetime_in_range_open_ended() {
        // No constraints
        let query = EventQuery::new();
        let dt = Local.with_ymd_and_hms(2025, 6, 4, 12, 0, 0).unwrap();
        assert!(query.datetime_in_range(&dt));

        // Only from_date
        let query = EventQuery {
            from_date: Some(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()),
            ..Default::default()
        };
        assert!(query.datetime_in_range(&Local.with_ymd_and_hms(2025, 6, 4, 12, 0, 0).unwrap()));
        assert!(query.datetime_in_range(&Local.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap()));
        assert!(!query.datetime_in_range(&Local.with_ymd_and_hms(2025, 5, 31, 0, 0, 0).unwrap()));
    }
}
