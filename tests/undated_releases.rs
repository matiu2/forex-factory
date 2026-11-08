//! Rows whose time cell names a release *window* rather than a time.
//!
//! Forex Factory writes a date range (`9th-14th`) or a date (`Oct 17th`) in
//! the time cell when a release has no fixed day. The week of 2026-11-08 had
//! three such low-impact rows (CNY M2 Money Supply, CNY New Loans, USD ADP
//! Weekly Employment Change), and strict evidence parsing rejected the whole
//! week over them — blocking high-impact evidence for every currency.
//!
//! They mean what "All Day" and "Tentative" mean: no exact time. Fatal on a
//! high-impact row, skipped otherwise.
use chrono::{NaiveDate, Utc};
use forex_factory::{CalendarParser, EventTiming};
use tracing_subscriber::prelude::*;

fn init() {
    tracing_subscriber::registry()
        .with(tracing_error::ErrorLayer::default())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init()
        .ok();
}

/// A timed high-impact release (13:30 UTC, 2026-11-06).
const VALID: &str = include_str!("fixtures/utc-release.html");

fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 6).unwrap()
}

fn parser() -> CalendarParser {
    CalendarParser::with_timezone("UTC").unwrap()
}

/// `VALID`'s row restyled with this impact icon and time text.
fn row(impact_class: &str, time_text: &str, id: &str) -> String {
    VALID
        .replace("icon--ff-impact-red", impact_class)
        .replace("1:30pm", time_text)
        .replace("test-release", id)
}

/// Kept, marked untimed — the same event display parsing returns, so strict
/// and display parsing agree on the week's events.
#[test]
fn low_and_medium_release_windows_are_kept_marked_untimed() {
    init();
    for impact in ["icon--ff-impact-yel", "icon--ff-impact-ora"] {
        for window in ["9th-14th", "Oct 17th"] {
            let html = format!("{VALID}{}", row(impact, window, "window"));
            let events = parser()
                .parse_high_impact_evidence(&html, date())
                .unwrap_or_else(|e| panic!("{impact} {window:?} rejected the week: {e}"));
            assert_eq!(events.len(), 2, "{impact} {window:?}");
            assert_eq!(
                events[0].datetime.with_timezone(&Utc).to_rfc3339(),
                "2026-11-06T13:30:00+00:00"
            );
            assert!(events[0].has_exact_time());
            assert_eq!(
                events[1].timing,
                EventTiming::Untimed {
                    label: window.into()
                }
            );
            assert_eq!(
                events[1].datetime.with_timezone(&Utc).to_rfc3339(),
                "2026-11-06T00:00:00+00:00"
            );
            assert_eq!(events, parser().parse(&html, date()));
        }
    }
}

#[test]
fn high_impact_release_window_is_still_rejected() {
    init();
    for window in ["9th-14th", "Oct 17th"] {
        let html = format!("{VALID}{}", row("icon--ff-impact-red", window, "window"));
        assert!(
            parser().parse_high_impact_evidence(&html, date()).is_err(),
            "a high-impact {window:?} has no exact time to build a window around"
        );
    }
}

/// A row with an empty time cell shares the time of the row above. After a
/// release window that is the window itself: the row below is untimed too,
/// at any impact, and never borrows a time from further up.
#[test]
fn release_window_is_shared_by_a_blank_row_below() {
    init();
    let window = row("icon--ff-impact-yel", "9th-14th", "window");
    let follower = row("icon--ff-impact-yel", "", "follower").replace("Fri Nov 6", "");
    let html = format!("{VALID}{window}{follower}");
    let events = parser()
        .parse_high_impact_evidence(&html, date())
        .expect("a blank row under an untimed one is untimed, not an error");
    assert_eq!(events.len(), 3);
    assert_eq!(
        events[2].timing,
        EventTiming::Untimed {
            label: "9th-14th".into()
        }
    );
    assert_eq!(
        events[2].datetime.with_timezone(&Utc).to_rfc3339(),
        "2026-11-06T00:00:00+00:00"
    );
    assert_eq!(events, parser().parse(&html, date()));
}
