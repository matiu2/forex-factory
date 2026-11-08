//! The week of 9 Nov 2026, as Forex Factory served it (UTC) for
//! `?week=nov9.2026`, saved through `HttpCalendarFetcher` on 2026-11-08.
//!
//! `?week=<date>` is the seven days starting at that date — checked live:
//! nov8 → Sun 8–Sat 14, nov9 → Mon 9–Sun 15, nov11 → Wed 11–Tue 17. So a
//! Monday request (what the trade-control worker sends on a Monday) carries
//! Sunday 15's rows, which the strict date check used to reject as "outside
//! the week".
use chrono::{NaiveDate, Utc};
use forex_factory::{CalendarParser, EconomicEvent, Impact};
use tracing_subscriber::prelude::*;

const PAGE: &str = include_str!("fixtures/ff_week_nov9_2026_utc.html");

fn init() {
    tracing_subscriber::registry()
        .with(tracing_error::ErrorLayer::default())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init()
        .ok();
}

fn monday() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
}

fn evidence() -> Vec<EconomicEvent> {
    init();
    CalendarParser::with_timezone("UTC")
        .unwrap()
        .parse_high_impact_evidence(PAGE, monday())
        .unwrap()
}

fn utc(event: &EconomicEvent) -> String {
    event.datetime.with_timezone(&Utc).to_rfc3339()
}

#[test]
fn strict_evidence_accepts_the_monday_page() {
    let high: Vec<_> = evidence()
        .iter()
        .filter(|e| e.impact == Impact::High)
        .map(|e| format!("{} {} {}", utc(e), e.currency, e.name))
        .collect();
    assert_eq!(
        high,
        [
            "2026-11-10T13:30:00+00:00 USD Core CPI m/m",
            // A blank time cell: same time as the row above.
            "2026-11-10T13:30:00+00:00 USD CPI m/m",
            "2026-11-12T07:00:00+00:00 GBP GDP m/m",
            "2026-11-13T13:30:00+00:00 USD Core PPI m/m",
            "2026-11-13T13:30:00+00:00 USD PPI m/m",
        ]
    );
}

#[test]
fn the_sunday_after_a_monday_is_inside_its_page() {
    let events = evidence();
    let services = events
        .iter()
        .find(|e| e.name == "BusinessNZ Services Index")
        .expect("Sunday 15's row");
    assert_eq!(utc(services), "2026-11-15T21:30:00+00:00");
}

#[test]
fn a_date_before_the_requested_day_is_not_the_requested_page() {
    init();
    let parser = CalendarParser::with_timezone("UTC").unwrap();
    // The page starts at the requested day, so Monday's rows are outside a
    // page requested for Tuesday.
    let tuesday = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
    assert!(parser.parse_high_impact_evidence(PAGE, tuesday).is_err());
}
