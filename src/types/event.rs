use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use super::Impact;

/// An economic event from the Forex Factory calendar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EconomicEvent {
    /// Event title (e.g., "Non-Farm Payrolls", "Interest Rate Decision")
    pub name: String,

    /// Currency affected (e.g., "USD", "EUR", "AUD")
    pub currency: String,

    /// Impact level on the market
    pub impact: Impact,

    /// Scheduled date and time of the event (local timezone)
    pub datetime: DateTime<Local>,

    /// Actual value (if released)
    pub actual: Option<String>,

    /// Forecasted value
    pub forecast: Option<String>,

    /// Previous period's value
    pub previous: Option<String>,
}

impl EconomicEvent {
    /// Check if this event matches the given minimum impact level
    pub fn meets_impact(&self, min_impact: Impact) -> bool {
        self.impact >= min_impact
    }

    /// Check if this event is for one of the given currencies
    pub fn matches_currencies(&self, currencies: &[String]) -> bool {
        if currencies.is_empty() {
            return true;
        }
        currencies
            .iter()
            .any(|c| c.eq_ignore_ascii_case(&self.currency))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample_event(currency: &str, impact: Impact) -> EconomicEvent {
        EconomicEvent {
            name: "Test Event".to_string(),
            currency: currency.to_string(),
            impact,
            datetime: Local.with_ymd_and_hms(2025, 6, 4, 12, 0, 0).unwrap(),
            actual: None,
            forecast: Some("1.5%".to_string()),
            previous: Some("1.2%".to_string()),
        }
    }

    #[test]
    fn test_meets_impact() {
        let high_event = sample_event("USD", Impact::High);
        let medium_event = sample_event("USD", Impact::Medium);
        let low_event = sample_event("USD", Impact::Low);

        // High impact meets all levels
        assert!(high_event.meets_impact(Impact::Low));
        assert!(high_event.meets_impact(Impact::Medium));
        assert!(high_event.meets_impact(Impact::High));

        // Medium impact meets low and medium
        assert!(medium_event.meets_impact(Impact::Low));
        assert!(medium_event.meets_impact(Impact::Medium));
        assert!(!medium_event.meets_impact(Impact::High));

        // Low impact only meets low
        assert!(low_event.meets_impact(Impact::Low));
        assert!(!low_event.meets_impact(Impact::Medium));
        assert!(!low_event.meets_impact(Impact::High));
    }

    #[test]
    fn test_matches_currencies() {
        let usd_event = sample_event("USD", Impact::High);

        // Empty list matches all
        assert!(usd_event.matches_currencies(&[]));

        // Exact match
        assert!(usd_event.matches_currencies(&["USD".to_string()]));

        // Case insensitive
        assert!(usd_event.matches_currencies(&["usd".to_string()]));

        // Multiple currencies
        assert!(usd_event.matches_currencies(&["EUR".to_string(), "USD".to_string()]));

        // No match
        assert!(!usd_event.matches_currencies(&["EUR".to_string(), "GBP".to_string()]));
    }
}
