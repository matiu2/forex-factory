//! Forex Factory's weekly JSON export — the list The5ers publish.
//!
//! `https://nfs.faireconomy.media/ff_calendar_thisweek.json` is an array of
//! `{title, country, date, impact, forecast, previous}`, `date` being RFC 3339
//! in New York time. It only ever holds the current Forex Factory week
//! (Sunday to Saturday, New York time).

use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate, TimeZone, Utc};
use chrono_tz::America::New_York;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::types::{EconomicEvent, EventTiming, Impact};

/// URL of the export.
pub const FEED_URL: &str = "https://nfs.faireconomy.media/ff_calendar_thisweek.json";

/// The weekly export — the list The5ers publish — with the span it covers:
/// its Forex Factory week, Sunday 00:00 to the next Sunday 00:00, New York
/// time.
///
/// Serializable so a caller can cache it: the export is rate-limited hard
/// (HTTP 429 after about five requests in a few minutes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct The5ersWeek {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    events: Vec<EconomicEvent>,
}

impl The5ersWeek {
    /// Parse the export. `Ok(None)` for an empty export: it covers no week.
    pub(crate) fn parse(json: &str) -> Result<Option<Self>> {
        let raw: Vec<FeedEvent> = serde_json::from_str(json)?;
        let Some(sunday) = raw.iter().map(|e| e.date.date_naive()).min().map(sunday_of) else {
            return Ok(None);
        };
        let start = new_york_midnight(sunday)?;
        let end = new_york_midnight(sunday + chrono::Days::new(7))?;
        let events = raw.into_iter().map(EconomicEvent::from).collect();
        Ok(Some(Self { start, end, events }))
    }

    /// Sunday 00:00 New York, the first instant the export covers.
    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    /// The next Sunday 00:00 New York, the first instant it no longer covers.
    pub fn end(&self) -> DateTime<Utc> {
        self.end
    }

    /// Every event of the week.
    pub fn events(&self) -> &[EconomicEvent] {
        &self.events
    }

    /// Whether the export can answer for all of `[from, until)` by itself.
    /// A window that runs outside the week, even by an hour, is not covered:
    /// the export knows nothing of the rest, so it must not answer for it.
    pub fn covers(&self, from: DateTime<Utc>, until: DateTime<Utc>) -> bool {
        self.start <= from && from <= until && until <= self.end
    }

    /// The events in `[from, until)`.
    pub fn events_between(&self, from: DateTime<Utc>, until: DateTime<Utc>) -> Vec<EconomicEvent> {
        self.events
            .iter()
            .filter(|e| (from..until).contains(&e.datetime.with_timezone(&Utc)))
            .cloned()
            .collect()
    }
}

/// 00:00 New York on `date`. Midnight always exists there: US clocks change
/// at 02:00.
fn new_york_midnight(date: NaiveDate) -> Result<DateTime<Utc>> {
    New_York
        .from_local_datetime(&date.and_time(chrono::NaiveTime::MIN))
        .earliest()
        .map(|at| at.with_timezone(&Utc))
        .ok_or_else(|| Error::InvalidEventTime(format!("{date} 00:00 in New York")))
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
            // The export's timestamp is what The5ers publish, untimed rows
            // included, so it is the instant their news rule is judged by.
            timing: EventTiming::Exact,
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

    fn fixture() -> The5ersWeek {
        The5ersWeek::parse(FIXTURE)
            .expect("fixture parses")
            .expect("fixture is not empty")
    }

    fn find<'a>(week: &'a The5ersWeek, currency: &str, name: &str) -> &'a EconomicEvent {
        week.events()
            .iter()
            .find(|e| e.currency == currency && e.name == name)
            .unwrap_or_else(|| panic!("{currency} {name} not in fixture"))
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

    fn utc(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
    }

    /// Sunday 1 Nov 00:00 New York is still EDT (clocks go back at 02:00);
    /// Sunday 8 Nov 00:00 is EST.
    #[test]
    fn spans_its_new_york_week() {
        let week = fixture();
        assert_eq!(week.start(), utc(2026, 11, 1, 4));
        assert_eq!(week.end(), utc(2026, 11, 8, 5));
    }

    #[test]
    fn covers_only_windows_entirely_inside_its_week() {
        let week = fixture();
        assert!(week.covers(week.start(), week.end()), "the whole week");
        assert!(
            week.covers(utc(2026, 11, 3, 12), utc(2026, 11, 4, 12)),
            "a day inside it"
        );
        assert!(
            !week.covers(utc(2026, 11, 1, 0), utc(2026, 11, 8, 0)),
            "a UTC Sunday page starts four hours before it"
        );
        assert!(
            !week.covers(utc(2026, 11, 2, 0), utc(2026, 11, 9, 0)),
            "a Monday page runs a day past it"
        );
        assert!(!week.covers(utc(2026, 11, 7, 12), utc(2026, 11, 8, 12)));
        assert!(
            !week.covers(utc(2026, 11, 4, 0), utc(2026, 11, 3, 0)),
            "inverted"
        );
    }

    #[test]
    fn events_between_is_half_open() {
        let week = fixture();
        let nfp = utc(2026, 11, 6, 13) + chrono::Duration::minutes(30);
        let names = |from, until| {
            week.events_between(from, until)
                .into_iter()
                .map(|e| e.name)
                .collect::<Vec<_>>()
        };
        assert!(
            names(nfp, nfp + chrono::Duration::minutes(1))
                .contains(&"Non-Farm Employment Change".to_owned())
        );
        assert!(
            !names(nfp - chrono::Duration::minutes(1), nfp)
                .contains(&"Non-Farm Employment Change".to_owned())
        );
    }

    /// A week with no Sunday events still starts on the Sunday.
    #[test]
    fn week_starts_on_sunday_even_without_sunday_events() {
        let json = r#"[{"title":"CPI m/m","country":"USD","date":"2026-11-11T08:30:00-05:00",
            "impact":"High","forecast":"0.3%","previous":"0.2%"}]"#;
        let week = The5ersWeek::parse(json).unwrap().unwrap();
        assert_eq!(week.start(), utc(2026, 11, 8, 5));
        assert_eq!(week.end(), utc(2026, 11, 15, 5));
    }

    /// The week is fixed in New York time: a Saturday-evening New York event
    /// is already Sunday in UTC, but must not push the week forward.
    #[test]
    fn week_is_judged_in_new_york_time() {
        let json = r#"[{"title":"Late","country":"USD","date":"2026-11-07T21:00:00-05:00",
            "impact":"Low","forecast":"","previous":""}]"#;
        let week = The5ersWeek::parse(json).unwrap().unwrap();
        assert_eq!(week.start(), utc(2026, 11, 1, 4));
    }

    /// Survives a cache round trip.
    #[test]
    fn serializes_for_caching() {
        let week = fixture();
        let json = serde_json::to_string(&week).unwrap();
        assert_eq!(serde_json::from_str::<The5ersWeek>(&json).unwrap(), week);
    }

    #[test]
    fn empty_export_covers_nothing() {
        assert_eq!(The5ersWeek::parse("[]").unwrap(), None);
    }

    /// When rate-limited the endpoint answers with an HTML page, not JSON.
    #[test]
    fn non_json_is_an_error() {
        assert!(The5ersWeek::parse("<html>Request Denied</html>").is_err());
    }
}
