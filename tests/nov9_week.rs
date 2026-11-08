//! The week of 9 Nov 2026, as Forex Factory served it (UTC) for
//! `?week=nov9.2026`, saved through `HttpCalendarFetcher` on 2026-11-08.
//!
//! `?week=<date>` is the seven days starting at that date — checked live:
//! nov8 → Sun 8–Sat 14, nov9 → Mon 9–Sun 15, nov11 → Wed 11–Tue 17. So a
//! Monday request (what the trade-control worker sends on a Monday) carries
//! Sunday 15's rows, which the strict date check used to reject as "outside
//! the week".
use chrono::{NaiveDate, Utc};
use forex_factory::{CalendarParser, EconomicEvent, EventTiming, Impact};
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

fn untimed(label: &str) -> EventTiming {
    EventTiming::Untimed {
        label: label.into(),
    }
}

/// Display parsing never borrows a time for a row whose cell is not a clock
/// time: the event is marked untimed and sits at midnight of its own day.
#[test]
fn lenient_parsing_marks_untimed_rows_instead_of_inventing_a_time() {
    init();
    let parser = CalendarParser::with_timezone("UTC").unwrap();
    for events in [
        parser.parse(PAGE, monday()),
        parser.parse_checked(PAGE, monday()).unwrap(),
    ] {
        let find = |currency: &str, name: &str| {
            events
                .iter()
                .find(|e| e.currency == currency && e.name == name)
                .unwrap_or_else(|| panic!("{currency} {name} missing"))
                .clone()
        };
        for (currency, name, label, day) in [
            ("CNY", "M2 Money Supply y/y", "9th-14th", "2026-11-09"),
            ("EUR", "ECOFIN Meetings", "All Day", "2026-11-09"),
            ("CNY", "Trade Balance", "Tentative", "2026-11-10"),
            ("USD", "Bank Holiday", "All Day", "2026-11-11"),
        ] {
            let event = find(currency, name);
            assert_eq!(event.timing, untimed(label), "{name}");
            assert!(!event.has_exact_time(), "{name}");
            assert_eq!(utc(&event), format!("{day}T00:00:00+00:00"), "{name}");
        }
        let cpi = find("USD", "CPI m/m");
        assert_eq!(cpi.timing, EventTiming::Exact);
        assert_eq!(utc(&cpi), "2026-11-10T13:30:00+00:00");
        assert!(events.iter().any(|e| e.timing == untimed("Oct 17th")));
    }
}

/// Strict and display parsing read time cells the same way: every event the
/// strict parser returns comes out of the display parser identical.
#[test]
fn lenient_and_strict_agree_on_every_strict_event() {
    let strict = evidence();
    init();
    let parser = CalendarParser::with_timezone("UTC").unwrap();
    let lenient = parser.parse(PAGE, monday());
    for event in &strict {
        assert!(
            lenient.contains(event),
            "{} {} differs",
            event.currency,
            event.name
        );
    }
}
