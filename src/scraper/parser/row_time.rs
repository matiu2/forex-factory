//! How a row's time cell is read — one interpretation shared by display and
//! evidence parsing, so both agree on which events have a release instant.
use super::parse_time;
use chrono::NaiveTime;

/// What the time cell itself says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Cell {
    /// A clock time, e.g. "1:30pm".
    Clock(NaiveTime),
    /// Empty: Forex Factory leaves the cell blank when a row shares the time
    /// of the row above.
    Inherit,
    /// Anything else: "All Day", "Tentative", "Day 2", "9th-14th", ...
    Untimed(String),
}

/// A row's resolved time — what a blank cell below it inherits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RowTime {
    Clock(NaiveTime),
    Untimed(String),
}

pub(super) fn classify(text: &str) -> Cell {
    let text = text.trim();
    if text.is_empty() {
        Cell::Inherit
    } else {
        parse_time(text).map_or_else(|| Cell::Untimed(text.to_owned()), Cell::Clock)
    }
}

/// Resolve `cell` against the time `current` rows on this day share, and
/// update it. `None` only for a blank cell with nothing above it that day.
pub(super) fn resolve(cell: &Cell, current: &mut Option<RowTime>) -> Option<RowTime> {
    match cell {
        Cell::Clock(time) => *current = Some(RowTime::Clock(*time)),
        Cell::Untimed(label) => *current = Some(RowTime::Untimed(label.clone())),
        Cell::Inherit => {}
    }
    current.clone()
}

/// Whether an untimed label is one of the forms Forex Factory writes for a
/// release with no exact time: "All Day", "Tentative", "Day 2", a day range
/// such as "9th-14th", or a date such as "Oct 17th". Anything else might be a
/// garbled clock time, which evidence must not take for "untimed".
pub(super) fn is_known_untimed(label: &str) -> bool {
    matches!(label, "All Day" | "Tentative")
        || label
            .strip_prefix("Day ")
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        || label
            .split_once('-')
            .is_some_and(|(from, to)| ordinal_day(from) && ordinal_day(to))
        || label
            .split_once(' ')
            .is_some_and(|(month, day)| MONTHS.contains(&month) && ordinal_day(day))
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// "1st", "22nd", "3rd", "14th" — a day of the month, 1 to 31.
fn ordinal_day(text: &str) -> bool {
    let digits = text.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let suffix = &text[digits.len()..];
    matches!(suffix, "st" | "nd" | "rd" | "th")
        && digits
            .parse::<u32>()
            .is_ok_and(|day| (1..=31).contains(&day))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn cells_are_clock_blank_or_untimed() {
        assert_eq!(classify("1:30pm"), Cell::Clock(clock(13, 30)));
        assert_eq!(classify("12:01am"), Cell::Clock(clock(0, 1)));
        assert_eq!(classify("  "), Cell::Inherit);
        for text in ["All Day", "Tentative", "Day 2", "9th-14th", "Oct 17th"] {
            assert_eq!(classify(text), Cell::Untimed(text.into()), "{text}");
        }
    }

    #[test]
    fn a_blank_cell_shares_the_row_above_whatever_it_was() {
        let mut current = None;
        assert_eq!(resolve(&Cell::Inherit, &mut current), None);
        let at = Cell::Clock(clock(7, 0));
        assert_eq!(
            resolve(&at, &mut current),
            Some(RowTime::Clock(clock(7, 0)))
        );
        assert_eq!(
            resolve(&Cell::Inherit, &mut current),
            Some(RowTime::Clock(clock(7, 0)))
        );
        let tentative = Cell::Untimed("Tentative".into());
        resolve(&tentative, &mut current);
        assert_eq!(
            resolve(&Cell::Inherit, &mut current),
            Some(RowTime::Untimed("Tentative".into())),
            "a blank row under an untimed one is untimed, never an earlier time"
        );
    }

    #[test]
    fn known_untimed_forms() {
        for label in [
            "All Day",
            "Tentative",
            "Day 1",
            "Day 12",
            "9th-14th",
            "1st-3rd",
            "Oct 17th",
            "Nov 2nd",
        ] {
            assert!(is_known_untimed(label), "{label}");
        }
        for label in [
            "",
            "Day",
            "Day x",
            "13:3O",
            "soon",
            "9th-",
            "Oct 32nd",
            "Okt 17th",
            "17th",
            "1:30 pm EST",
        ] {
            assert!(!is_known_untimed(label), "{label}");
        }
    }
}
