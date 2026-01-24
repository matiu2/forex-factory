use chrono::{DateTime, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use super::{Currency, Impact};

/// Query parameters for filtering economic events.
///
/// # Builder Pattern
///
/// Use the typed builder for compile-time checked construction:
///
/// ```
/// use forex_factory::{EventQuery, Impact, Currency};
/// use chrono::NaiveDate;
///
/// let query = EventQuery::builder()
///     .currencies(vec![Currency::EUR, Currency::USD])
///     .min_impact(Impact::High)
///     .from_date(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap())
///     .to_date(NaiveDate::from_ymd_opt(2024, 6, 7).unwrap())
///     .build();
/// ```
///
/// # Fluent API
///
/// Or use the fluent methods for ergonomic construction:
///
/// ```
/// use forex_factory::{EventQuery, Impact};
///
/// let query = EventQuery::new()
///     .with_currency_pair("EUR/USD")
///     .with_min_impact(Impact::Medium);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, TypedBuilder)]
#[builder(field_defaults(default))]
pub struct EventQuery {
    /// Filter by currencies (empty means all currencies)
    #[serde(default)]
    #[builder(default)]
    pub currencies: Vec<Currency>,

    /// Start date for the query range (inclusive)
    #[builder(default, setter(strip_option))]
    pub from_date: Option<NaiveDate>,

    /// End date for the query range (inclusive)
    #[builder(default, setter(strip_option))]
    pub to_date: Option<NaiveDate>,

    /// Start datetime for precise time filtering (inclusive)
    /// Takes precedence over from_date for filtering
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub from_datetime: Option<DateTime<Local>>,

    /// End datetime for precise time filtering (inclusive)
    /// Takes precedence over to_date for filtering
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub to_datetime: Option<DateTime<Local>>,

    /// Minimum impact level (None means all impacts)
    #[builder(default, setter(strip_option))]
    pub min_impact: Option<Impact>,
}

impl EventQuery {
    /// Create a new empty query (matches all events)
    pub fn new() -> Self {
        Self::default()
    }

    /// Set currencies from a currency pair string (e.g., "AUD/CHF", "EUR-USD")
    ///
    /// Extracts both currencies from the pair. Unrecognized currency codes are ignored.
    pub fn with_currency_pair(mut self, pair: &str) -> Self {
        self.currencies = pair
            .split(['/', '-', '_'])
            .filter_map(|s| s.trim().parse::<Currency>().ok())
            .collect();
        self
    }

    /// Set a single currency
    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.currencies = vec![currency];
        self
    }

    /// Set multiple currencies
    pub fn with_currencies(mut self, currencies: impl IntoIterator<Item = Currency>) -> Self {
        self.currencies = currencies.into_iter().collect();
        self
    }

    /// Add currencies without replacing existing ones
    pub fn add_currency(mut self, currency: Currency) -> Self {
        if !self.currencies.contains(&currency) {
            self.currencies.push(currency);
        }
        self
    }

    /// Add multiple currencies without replacing existing ones
    pub fn add_currencies(mut self, currencies: impl IntoIterator<Item = Currency>) -> Self {
        for currency in currencies {
            if !self.currencies.contains(&currency) {
                self.currencies.push(currency);
            }
        }
        self
    }

    /// Set the date range (dates only, time is ignored)
    pub fn with_date_range(mut self, from: NaiveDate, to: NaiveDate) -> Self {
        self.from_date = Some(from);
        self.to_date = Some(to);
        self
    }

    /// Set precise datetime range
    ///
    /// Also sets from_date/to_date for fetch optimization.
    pub fn with_datetime_range(mut self, from: DateTime<Local>, to: DateTime<Local>) -> Self {
        self.from_datetime = Some(from);
        self.to_datetime = Some(to);
        self.from_date = Some(from.date_naive());
        self.to_date = Some(to.date_naive());
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

    /// Check if an event's datetime falls within the query range
    ///
    /// Uses datetime range if set, otherwise falls back to date range.
    pub fn datetime_in_range(&self, datetime: &DateTime<Local>) -> bool {
        // Check datetime range first (more precise)
        if let Some(from) = self.from_datetime {
            if *datetime < from {
                return false;
            }
        } else if let Some(from) = self.from_date
            && datetime.date_naive() < from
        {
            return false;
        }

        if let Some(to) = self.to_datetime {
            if *datetime > to {
                return false;
            }
        } else if let Some(to) = self.to_date
            && datetime.date_naive() > to
        {
            return false;
        }

        true
    }

    /// Check if a currency matches the query filter
    ///
    /// Returns true if currencies list is empty (match all) or contains the currency.
    pub fn matches_currency(&self, currency: &Currency) -> bool {
        self.currencies.is_empty() || self.currencies.contains(currency)
    }

    /// Check if a currency code string matches the query filter
    ///
    /// Parses the string to Currency first. Returns true if currencies list is empty.
    pub fn matches_currency_str(&self, currency_str: &str) -> bool {
        if self.currencies.is_empty() {
            return true;
        }
        currency_str
            .parse::<Currency>()
            .map(|c| self.currencies.contains(&c))
            .unwrap_or(false)
    }

    /// Get the date range needed for fetching
    ///
    /// Accounts for datetime range by extracting dates.
    pub fn fetch_date_range(&self) -> (Option<NaiveDate>, Option<NaiveDate>) {
        let from = self
            .from_datetime
            .map(|dt| dt.date_naive())
            .or(self.from_date);
        let to = self.to_datetime.map(|dt| dt.date_naive()).or(self.to_date);
        (from, to)
    }
}

impl AsRef<EventQuery> for EventQuery {
    fn as_ref(&self) -> &EventQuery {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_with_currency_pair() {
        let query = EventQuery::new().with_currency_pair("AUD/CHF");
        assert_eq!(query.currencies, vec![Currency::AUD, Currency::CHF]);

        let query = EventQuery::new().with_currency_pair("EUR-USD");
        assert_eq!(query.currencies, vec![Currency::EUR, Currency::USD]);

        let query = EventQuery::new().with_currency_pair("gbp_jpy");
        assert_eq!(query.currencies, vec![Currency::GBP, Currency::JPY]);
    }

    #[test]
    fn test_with_single_currency() {
        let query = EventQuery::new().with_currency(Currency::USD);
        assert_eq!(query.currencies, vec![Currency::USD]);
    }

    #[test]
    fn test_add_currencies() {
        let query = EventQuery::new()
            .add_currency(Currency::EUR)
            .add_currency(Currency::USD)
            .add_currency(Currency::EUR); // Duplicate should be ignored

        assert_eq!(query.currencies, vec![Currency::EUR, Currency::USD]);
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
    fn test_datetime_range_precise() {
        let from = Local.with_ymd_and_hms(2025, 6, 3, 13, 0, 0).unwrap();
        let to = Local.with_ymd_and_hms(2025, 6, 4, 13, 0, 0).unwrap();
        let query = EventQuery::new().with_datetime_range(from, to);

        // Within range
        assert!(query.datetime_in_range(&Local.with_ymd_and_hms(2025, 6, 3, 14, 0, 0).unwrap()));
        assert!(query.datetime_in_range(&Local.with_ymd_and_hms(2025, 6, 4, 12, 0, 0).unwrap()));

        // Exactly at boundaries
        assert!(query.datetime_in_range(&from));
        assert!(query.datetime_in_range(&to));

        // Outside range
        assert!(!query.datetime_in_range(&Local.with_ymd_and_hms(2025, 6, 3, 12, 59, 0).unwrap()));
        assert!(!query.datetime_in_range(&Local.with_ymd_and_hms(2025, 6, 4, 13, 1, 0).unwrap()));
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

    #[test]
    fn test_matches_currency() {
        let query = EventQuery::new().with_currency_pair("EUR/USD");

        assert!(query.matches_currency(&Currency::EUR));
        assert!(query.matches_currency(&Currency::USD));
        assert!(!query.matches_currency(&Currency::GBP));
    }

    #[test]
    fn test_matches_currency_str() {
        let query = EventQuery::new().with_currency_pair("EUR/USD");

        assert!(query.matches_currency_str("EUR"));
        assert!(query.matches_currency_str("usd"));
        assert!(!query.matches_currency_str("GBP"));
        assert!(!query.matches_currency_str("INVALID"));
    }

    #[test]
    fn test_empty_currencies_matches_all() {
        let query = EventQuery::new();

        assert!(query.matches_currency(&Currency::EUR));
        assert!(query.matches_currency(&Currency::JPY));
        assert!(query.matches_currency_str("CAD"));
    }

    #[test]
    fn test_typed_builder() {
        let query = EventQuery::builder()
            .currencies(vec![Currency::EUR, Currency::USD])
            .min_impact(Impact::High)
            .from_date(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap())
            .build();

        assert_eq!(query.currencies, vec![Currency::EUR, Currency::USD]);
        assert_eq!(query.min_impact, Some(Impact::High));
        assert_eq!(
            query.from_date,
            Some(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap())
        );
    }

    #[test]
    fn test_fetch_date_range() {
        // With datetime range
        let from = Local.with_ymd_and_hms(2025, 6, 3, 13, 0, 0).unwrap();
        let to = Local.with_ymd_and_hms(2025, 6, 4, 13, 0, 0).unwrap();
        let query = EventQuery::new().with_datetime_range(from, to);

        let (start, end) = query.fetch_date_range();
        assert_eq!(start, Some(NaiveDate::from_ymd_opt(2025, 6, 3).unwrap()));
        assert_eq!(end, Some(NaiveDate::from_ymd_opt(2025, 6, 4).unwrap()));

        // With date range only
        let query = EventQuery::new().with_date_range(
            NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 6, 7).unwrap(),
        );

        let (start, end) = query.fetch_date_range();
        assert_eq!(start, Some(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()));
        assert_eq!(end, Some(NaiveDate::from_ymd_opt(2025, 6, 7).unwrap()));
    }
}
