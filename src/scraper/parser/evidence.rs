//! Validation for high-impact entry restrictions, separate from display parsing.
use super::*;

pub(super) fn date(text: &str, reference: NaiveDate) -> Option<NaiveDate> {
    // `?week=<date>` is the seven days STARTING at that date, not the
    // Sunday week around it (checked live 2026-11-08: nov8 → Sun–Sat,
    // nov9 → Mon–Sun, nov11 → Wed–Tue), and the fetcher requests the
    // reference date itself.
    let start = reference;
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
    if text.is_empty() {
        return previous.map(Some).ok_or_else(|| {
            Error::InvalidCalendarEvidence("event time cannot be inherited from this day".into())
        });
    }
    // Anything but a clock time means "no exact time": "All Day",
    // "Tentative", or a release window such as "9th-14th" or "Oct 17th".
    // Fatal on a high-impact row — there is nothing to build a window around
    // — and skipped otherwise, so one undated low-impact release cannot void
    // a week's evidence. Either way, rows below must not inherit a time.
    let Some(time) = parse_time(text) else {
        *previous = None;
        return if impact == Impact::High {
            Err(Error::InvalidCalendarEvidence(format!(
                "high-impact event has no exact release time ({text:?})"
            )))
        } else {
            Ok(None)
        };
    };
    *previous = Some(time);
    Ok(Some(time))
}
