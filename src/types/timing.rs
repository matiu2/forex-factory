use serde::{Deserialize, Serialize};

/// Whether an event's [`datetime`](crate::EconomicEvent::datetime) is a real
/// release instant.
///
/// Forex Factory writes something other than a clock time in the time cell of
/// many rows: "All Day" (holidays, elections, meetings), "Tentative", "Day 2",
/// or a release window such as "9th-14th" or "Oct 17th". Those events have no
/// release instant, and display and evidence parsing both mark them
/// [`Untimed`](Self::Untimed) rather than giving them a time.
///
/// The weekly export ([`EventSource::PreferThe5ers`](crate::EventSource))
/// gives every event a concrete timestamp, untimed ones included, and that
/// timestamp is what The5ers publish; its events are always `Exact`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventTiming {
    /// `datetime` is the release instant.
    #[default]
    Exact,
    /// No release instant. `datetime` is midnight (in the parser's timezone)
    /// of the row's day: a placeholder that orders the event, never a time to
    /// build a news window around.
    Untimed {
        /// The time cell's text, e.g. "All Day" or "9th-14th". Empty when a
        /// row had no time of its own and nothing above it to share.
        label: String,
    },
}

impl EventTiming {
    pub fn is_exact(&self) -> bool {
        matches!(self, Self::Exact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cached events written before this field existed read back as `Exact`,
    /// and an exact event serializes without the field.
    #[test]
    fn exact_is_the_default_and_is_not_written() {
        assert_eq!(EventTiming::default(), EventTiming::Exact);
        #[derive(Serialize, Deserialize)]
        struct Holder {
            #[serde(default, skip_serializing_if = "EventTiming::is_exact")]
            timing: EventTiming,
        }
        let old: Holder = serde_json::from_str("{}").unwrap();
        assert_eq!(old.timing, EventTiming::Exact);
        let json = serde_json::to_string(&Holder {
            timing: EventTiming::Exact,
        })
        .unwrap();
        assert_eq!(json, "{}");
        let untimed = Holder {
            timing: EventTiming::Untimed {
                label: "All Day".into(),
            },
        };
        let json = serde_json::to_string(&untimed).unwrap();
        let back: Holder = serde_json::from_str(&json).unwrap();
        assert_eq!(back.timing, untimed.timing);
    }
}
