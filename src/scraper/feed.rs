//! Forex Factory's weekly JSON export — the list The5ers publish.
//!
//! `https://nfs.faireconomy.media/ff_calendar_thisweek.json` is an array of
//! `{title, country, date, impact, forecast, previous}`, `date` being RFC 3339
//! in New York time. It only ever holds the current Forex Factory week
//! (Sunday to Saturday, New York time).

use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate};
use serde::Deserialize;

use crate::error::Result;
use crate::types::{EconomicEvent, Impact};

/// URL of the export.
pub const FEED_URL: &str = "https://nfs.faireconomy.media/ff_calendar_thisweek.json";

/// The export's contents, with the week it covers.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FeedWeek {
    /// The Sunday the export's week starts on.
    week_start: NaiveDate,
    events: Vec<EconomicEvent>,
}

impl FeedWeek {
    /// Parse the export. `Ok(None)` for an empty export: it covers no week.
    pub(crate) fn parse(json: &str) -> Result<Option<Self>> {
        let raw: Vec<FeedEvent> = serde_json::from_str(json)?;
        let Some(week_start) = raw.iter().map(|e| e.date.date_naive()).min().map(sunday_of) else {
            return Ok(None);
        };
        let events = raw.into_iter().map(EconomicEvent::from).collect();
        Ok(Some(Self { week_start, events }))
    }

    /// Whether `date` falls in the export's week — i.e. whether the export,
    /// rather than the scraper, can answer a request for `date`'s week.
    pub(crate) fn covers(&self, date: NaiveDate) -> bool {
        sunday_of(date) == self.week_start
    }

    pub(crate) fn events(&self) -> &[EconomicEvent] {
        &self.events
    }
}

/// One element of the export, as served.
#[derive(Debug, Deserialize)]
struct FeedEvent {
    title: String,
    country: String,
    /// Kept in its own (New York) offset so the week is judged in New York
    /// time, the way Forex Factory delimits its weeks.
    date: DateTime<FixedOffset>,
    impact: String,
    #[serde(default)]
    forecast: String,
    #[serde(default)]
    previous: String,
}

impl From<FeedEvent> for EconomicEvent {
    fn from(e: FeedEvent) -> Self {
        Self {
            name: e.title,
            currency: e.country,
            impact: feed_impact(&e.impact),
            datetime: e.date.with_timezone(&Local),
            actual: None,
            forecast: non_empty(e.forecast),
            previous: non_empty(e.previous),
            event_id: None,
        }
    }
}

/// `High` / `Medium` map across; everything else (`Low`, `Holiday`,
/// `Non-Economic`) is `Low`, which is what the scraper does with the
/// grey "holiday" icon too.
fn feed_impact(impact: &str) -> Impact {
    match impact {
        "High" => Impact::High,
        "Medium" => Impact::Medium,
        _ => Impact::Low,
    }
}

fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

/// The Sunday on or before `date`: Forex Factory weeks run Sunday–Saturday.
fn sunday_of(date: NaiveDate) -> NaiveDate {
    date - chrono::Days::new(date.weekday().num_days_from_sunday().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    /// The real export for the week of 2026-11-01, captured 2026-11-08 and
    /// verified identical to The5ers' calendar page at the time.
    const FIXTURE: &str = include_str!("../../tests/fixtures/ff_calendar_week_2026-11-01.json");

    fn fixture() -> FeedWeek {
        FeedWeek::parse(FIXTURE)
            .expect("fixture parses")
            .expect("fixture is not empty")
    }

    fn find<'a>(week: &'a FeedWeek, currency: &str, name: &str) -> &'a EconomicEvent {
        week.events()
            .iter()
            .find(|e| e.currency == currency && e.name == name)
            .unwrap_or_else(|| panic!("{currency} {name} not in fixture"))
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn parses_every_event_and_impact() {
        let week = fixture();
        assert_eq!(week.events().len(), 116);
        let high = week.events().iter().filter(|e| e.impact == Impact::High);
        assert_eq!(high.count(), 19, "same 19 as the scraper found that week");
    }

    #[test]
    fn maps_fields_of_a_timed_event() {
        let week = fixture();
        let nfp = find(&week, "USD", "Non-Farm Employment Change");
        assert_eq!(nfp.impact, Impact::High);
        assert_eq!(
            nfp.datetime.with_timezone(&Utc),
            Utc.with_ymd_and_hms(2026, 11, 6, 13, 30, 0).unwrap(),
            "08:30 New York (EST) is 13:30 UTC"
        );
        assert_eq!(nfp.actual, None, "the export carries no actuals");
        assert_eq!(nfp.event_id, None, "the export carries no ids");
    }

    #[test]
    fn empty_values_become_none() {
        let week = fixture();
        let dst = find(&week, "USD", "Daylight Saving Time Shift");
        assert_eq!(dst.forecast, None);
        assert_eq!(dst.previous, None);

        let building = find(&week, "AUD", "Building Approvals m/m");
        assert_eq!(building.forecast.as_deref(), Some("1.8%"));
        assert_eq!(building.previous.as_deref(), Some("-6.1%"));
    }

    #[test]
    fn holidays_are_low_impact_like_the_scraper() {
        let week = fixture();
        assert_eq!(
            find(&week, "USD", "Daylight Saving Time Shift").impact,
            Impact::Low
        );
    }

    /// The reason this source exists: The5ers' time for an all-day event is
    /// the export's timestamp (10:16 New York here), not a time the scraper
    /// would produce.
    #[test]
    fn all_day_events_keep_the_exports_timestamp() {
        let week = fixture();
        let elections = find(&week, "USD", "Congressional Elections");
        assert_eq!(
            elections.datetime.with_timezone(&Utc),
            Utc.with_ymd_and_hms(2026, 11, 3, 15, 16, 0).unwrap()
        );
    }

    #[test]
    fn covers_its_sunday_to_saturday_week_only() {
        let week = fixture();
        assert!(!week.covers(date(2026, 10, 31)), "Saturday before");
        assert!(week.covers(date(2026, 11, 1)), "Sunday start");
        assert!(week.covers(date(2026, 11, 2)), "Monday");
        assert!(week.covers(date(2026, 11, 7)), "Saturday end");
        assert!(!week.covers(date(2026, 11, 8)), "next Sunday");
        assert!(!week.covers(date(2026, 11, 9)), "next Monday");
    }

    /// A week with no Sunday events still starts on the Sunday.
    #[test]
    fn week_starts_on_sunday_even_without_sunday_events() {
        let json = r#"[{"title":"CPI m/m","country":"USD","date":"2026-11-11T08:30:00-05:00",
            "impact":"High","forecast":"0.3%","previous":"0.2%"}]"#;
        let week = FeedWeek::parse(json).unwrap().unwrap();
        assert!(week.covers(date(2026, 11, 8)));
        assert!(week.covers(date(2026, 11, 14)));
        assert!(!week.covers(date(2026, 11, 7)));
    }

    /// The week is fixed in New York time: a Saturday-evening New York event
    /// is already Sunday in UTC, but must not push the week forward.
    #[test]
    fn week_is_judged_in_new_york_time() {
        let json = r#"[{"title":"Late","country":"USD","date":"2026-11-07T21:00:00-05:00",
            "impact":"Low","forecast":"","previous":""}]"#;
        let week = FeedWeek::parse(json).unwrap().unwrap();
        assert!(week.covers(date(2026, 11, 1)));
        assert!(!week.covers(date(2026, 11, 8)));
    }

    #[test]
    fn empty_export_covers_nothing() {
        assert_eq!(FeedWeek::parse("[]").unwrap(), None);
    }

    /// When rate-limited the endpoint answers with an HTML page, not JSON.
    #[test]
    fn non_json_is_an_error() {
        assert!(FeedWeek::parse("<html>Request Denied</html>").is_err());
    }
}
