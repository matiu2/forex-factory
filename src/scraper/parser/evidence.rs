//! Validation for high-impact entry restrictions, separate from display parsing.
use super::*;

pub(super) fn date(text: &str, reference: NaiveDate) -> Option<NaiveDate> {
    // Forex Factory's weekly calendar runs Sunday through Saturday:
    // https://www.forexfactory.com/calendar?week=nov1.2026
    let start = reference.checked_sub_signed(chrono::Duration::days(i64::from(
        reference.weekday().num_days_from_sunday(),
    )))?;
    let end = start.checked_add_signed(chrono::Duration::days(6))?;
    [reference.year() - 1, reference.year(), reference.year() + 1]
        .into_iter()
        .filter_map(|year| parse_date(text, year))
        .min_by_key(|date| date.signed_duration_since(reference).num_days().abs())
        .filter(|date| *date >= start && *date <= end)
}

pub(super) fn impact(
    parser: &CalendarParser,
    row: &scraper::ElementRef<'_>,
) -> Result<Option<Impact>> {
    let classes = row
        .select(&parser.impact_selector)
        .filter_map(|element| element.value().attr("class"))
        .flat_map(str::split_whitespace)
        .filter(|class| class.starts_with("icon--ff-impact-"))
        .collect::<Vec<_>>();
    match classes.as_slice() {
        ["icon--ff-impact-red"] => Ok(Some(Impact::High)),
        ["icon--ff-impact-ora"] => Ok(Some(Impact::Medium)),
        ["icon--ff-impact-yel"] => Ok(Some(Impact::Low)),
        ["icon--ff-impact-gra"] => Ok(None),
        _ => Err(Error::InvalidCalendarEvidence(
            "missing, ambiguous or unknown event impact".into(),
        )),
    }
}

pub(super) fn time(
    text: &str,
    previous: &mut Option<NaiveTime>,
    impact: Impact,
) -> Result<Option<NaiveTime>> {
    if matches!(text, "All Day" | "Tentative") {
        *previous = None;
        return if impact == Impact::High {
            Err(Error::InvalidCalendarEvidence(
                "high-impact event has no exact release time".into(),
            ))
        } else {
            Ok(None)
        };
    }
    if !text.is_empty() {
        *previous = Some(parse_time(text).ok_or_else(|| {
            Error::InvalidCalendarEvidence("unrecognized explicit event time".into())
        })?);
    }
    previous.map(Some).ok_or_else(|| {
        Error::InvalidCalendarEvidence("event time cannot be inherited from this day".into())
    })
}
