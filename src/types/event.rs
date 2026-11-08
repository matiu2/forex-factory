use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use super::{Currency, EventTiming, Impact};

/// An economic event from the Forex Factory calendar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EconomicEvent {
    /// Event title (e.g., "Non-Farm Payrolls", "Interest Rate Decision")
    pub name: String,

    /// Currency affected (e.g., "USD", "EUR", "AUD")
    pub currency: String,

    /// Impact level on the market
    pub impact: Impact,

    /// Scheduled date and time of the event (local timezone). Only a release
    /// instant when [`timing`](Self::timing) is `Exact`.
    pub datetime: DateTime<Local>,

    /// Whether `datetime` is a real release instant — see [`EventTiming`].
    #[serde(default, skip_serializing_if = "EventTiming::is_exact")]
    pub timing: EventTiming,

    /// Actual value (if released)
    pub actual: Option<String>,

    /// Forecasted value
    pub forecast: Option<String>,

    /// Previous period's value
    pub previous: Option<String>,

    /// Forex Factory's own id for this event, from the row's
    /// `data-event-id` attribute.
    ///
    /// This is the only stable per-event handle the calendar exposes — the
    /// rows carry no anchor, because the detail view is driven by JavaScript.
    /// It deep-links: `https://www.forexfactory.com/calendar?event=<id>`
    /// redirects to that event's day and opens its detail panel.
    ///
    /// `Option` deliberately: if the attribute ever moves or is renamed, an
    /// event must still parse WITHOUT an id rather than be dropped. Losing a
    /// high-impact print because its id changed shape would be far worse than
    /// losing a link to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
}

impl EconomicEvent {
    /// A deep link to this event on forexfactory.com, or `None` when the
    /// event carries no [`event_id`](Self::event_id).
    ///
    /// `?event=<id>` is a redirect: Forex Factory resolves it to the event's
    /// own day and opens its detail panel
    /// (`?day=sep20.2026&event=150129#detail=150129`). Verified against the
    /// live site — `/calendar/detail/<id>` does NOT work, and the row carries
    /// no anchor of its own, so this is the form to use.
    ///
    /// Built here rather than by each caller so the URL shape is stated once;
    /// a caller that hand-assembled it would be a second thing to keep in step.
    pub fn detail_url(&self) -> Option<String> {
        self.event_id
            .as_deref()
            .map(|id| format!("https://www.forexfactory.com/calendar?event={id}"))
    }

    /// Whether `datetime` is the release instant (not an "All Day",
    /// "Tentative" or release-window placeholder).
    pub fn has_exact_time(&self) -> bool {
        self.timing.is_exact()
    }

    /// Check if this event matches the given minimum impact level
    pub fn meets_impact(&self, min_impact: Impact) -> bool {
        self.impact >= min_impact
    }

    /// Get the currency as a Currency enum, if it's a known currency
    pub fn currency_enum(&self) -> Option<Currency> {
        self.currency.parse().ok()
    }

    /// Check if this event matches one of the given Currency enums
    pub fn matches_currency_list(&self, currencies: &[Currency]) -> bool {
        if currencies.is_empty() {
            return true;
        }
        self.currency_enum()
            .map(|c| currencies.contains(&c))
            .unwrap_or(false)
    }

    /// Check if this event is for one of the given currencies (string version)
    ///
    /// Deprecated: prefer `matches_currency_list` with Currency enums
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
            timing: EventTiming::Exact,
            actual: None,
            forecast: Some("1.5%".to_string()),
            previous: Some("1.2%".to_string()),
            event_id: None,
        }
    }

    #[test]
    fn detail_url_is_built_from_the_event_id() {
        let mut ev = sample_event("USD", Impact::High);
        assert_eq!(ev.detail_url(), None, "no id means no link");

        ev.event_id = Some("150129".to_string());
        assert_eq!(
            ev.detail_url().as_deref(),
            Some("https://www.forexfactory.com/calendar?event=150129"),
            "the `?event=` form is the one that resolves — see `detail_url`"
        );
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
