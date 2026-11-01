use chrono::{NaiveDate, Utc};
use forex_factory::CalendarParser;
use tracing_subscriber::prelude::*;
fn init() {
    tracing_subscriber::registry()
        .with(tracing_error::ErrorLayer::default())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init()
        .ok();
}
#[test]
fn explicit_calendar_timezone_is_independent_of_host_timezone() {
    init();
    let date = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
    for (timezone, expected) in [
        ("UTC", "2026-11-06T13:30:00+00:00"),
        ("Australia/Brisbane", "2026-11-06T03:30:00+00:00"),
        ("America/New_York", "2026-11-06T18:30:00+00:00"),
    ] {
        let parser = CalendarParser::with_timezone(timezone).unwrap();
        let events = parser
            .parse_checked(include_str!("fixtures/utc-release.html"), date)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].datetime.with_timezone(&Utc).to_rfc3339(),
            expected,
            "requested zone {timezone}"
        );
    }
}
#[test]
fn invalid_explicit_timezone_is_a_configuration_error() {
    init();
    assert!(CalendarParser::with_timezone("Not/A_Zone").is_err());
    assert!(forex_factory::CalendarService::with_timezone("Not/A_Zone").is_err());
}

#[test]
fn strict_timezone_parsing_rejects_partial_calendars_at_dst_transitions() {
    init();
    let parser = CalendarParser::with_timezone("America/New_York").unwrap();
    for (date_text, date, valid_time, invalid_time) in [
        (
            "Sun Nov 1",
            NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
            "12:30am",
            "1:30am",
        ),
        (
            "Sun Mar 8",
            NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(),
            "1:30am",
            "2:30am",
        ),
    ] {
        let valid = include_str!("fixtures/utc-release.html")
            .replace("Fri Nov 6", date_text)
            .replace("1:30pm", valid_time);
        let invalid = valid
            .replace(valid_time, invalid_time)
            .replace("test-release", "invalid-time");
        let mixed = format!("{valid}{invalid}");
        assert!(
            parser.parse_checked(&mixed, date).is_err(),
            "must not return partial evidence for {date_text} {invalid_time}"
        );
    }
}

#[test]
fn high_impact_evidence_rejects_malformed_rows_alongside_valid_events() {
    init();
    let parser = CalendarParser::with_timezone("UTC").unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
    let valid = include_str!("fixtures/utc-release.html");
    for damaged in [
        valid.replace("icon--ff-impact-red", "unknown-impact"),
        valid.replace("icon--ff-impact-red", "icon--ff-impact-yel-redesign"),
        valid.replace("Fri Nov 6", "éééNov 6"),
        valid.replace("1:30pm", "not a time"),
        valid.replace("Fri Nov 6", "invalid date"),
        valid.replace(">USD<", "><"),
        valid.replace(">USD<", ">US D<"),
        valid.replace(">USD<", ">USD ?<"),
        valid.replace("Fri Nov 6", "Fri Oct 30"),
        valid.replace("Employment release", ""),
        valid.replace("1:30pm", "Tentative"),
    ] {
        assert!(
            parser
                .parse_high_impact_evidence(&format!("{valid}{damaged}"), date)
                .is_err()
        );
    }
    let no_time = valid.replace("1:30pm", "");
    assert!(parser.parse_high_impact_evidence(&no_time, date).is_err());
    let same_release = no_time
        .replace("Fri Nov 6", "")
        .replace("test-release", "simultaneous");
    assert_eq!(
        parser
            .parse_high_impact_evidence(&format!("{valid}{same_release}"), date)
            .unwrap()
            .len(),
        2
    );
    let global_low = valid
        .replace(">USD<", ">All<")
        .replace("icon--ff-impact-red", "icon--ff-impact-yel")
        .replace("1:30pm", "All Day");
    assert_eq!(
        parser
            .parse_high_impact_evidence(&format!("{valid}{global_low}"), date)
            .unwrap()
            .len(),
        1
    );
    let global_high = valid.replace(">USD<", ">All<");
    assert_eq!(
        parser
            .parse_high_impact_evidence(&global_high, date)
            .unwrap()[0]
            .currency,
        "All"
    );
    assert!(
        parser
            .parse_high_impact_evidence(&global_high.replace("1:30pm", "Tentative"), date)
            .is_err()
    );
    let holiday = valid
        .replace("icon--ff-impact-red", "icon--ff-impact-gra")
        .replace("1:30pm", "All Day");
    let events = parser
        .parse_high_impact_evidence(&format!("{valid}{holiday}"), date)
        .unwrap();
    assert_eq!(events.len(), 1);
}

#[test]
fn strict_evidence_resolves_new_year_dates_within_the_requested_week() {
    init();
    let parser = CalendarParser::with_timezone("UTC").unwrap();
    for (base, text, expected) in [
        ((2026, 12, 31), "Fri Jan 1", "2027-01-01T13:30:00+00:00"),
        ((2027, 1, 1), "Thu Dec 31", "2026-12-31T13:30:00+00:00"),
    ] {
        let html = include_str!("fixtures/utc-release.html").replace("Fri Nov 6", text);
        let events = parser
            .parse_high_impact_evidence(
                &html,
                NaiveDate::from_ymd_opt(base.0, base.1, base.2).unwrap(),
            )
            .unwrap();
        assert_eq!(
            events[0].datetime.with_timezone(&Utc).to_rfc3339(),
            expected
        );
    }
}
