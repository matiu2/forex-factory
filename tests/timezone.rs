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
