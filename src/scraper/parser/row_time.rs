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
}
