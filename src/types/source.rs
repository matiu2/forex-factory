/// Where [`CalendarService`](crate::CalendarService) gets its events from.
///
/// Both choices are Forex Factory data; they differ in *which* Forex Factory
/// output is read, and that difference shows up in event times.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum EventSource {
    /// Scrape the forexfactory.com calendar HTML. Works for any week, past or
    /// future, and carries each event's [`event_id`](crate::EconomicEvent::event_id).
    #[default]
    ForexFactory,

    /// Use Forex Factory's weekly JSON export — the exact list The5ers
    /// publishes as their economic calendar — whenever it covers the requested
    /// week, and fall back to [`ForexFactory`](Self::ForexFactory) otherwise.
    ///
    /// Choose this when trading The5ers accounts: they count trades within
    /// ±2 minutes of a high-impact event as a breach, so the event times must
    /// be *theirs*. The two outputs agree on timed events, but not on "All
    /// Day" ones: the export gives those a concrete (and seemingly arbitrary)
    /// timestamp, while the scraper borrows the previous row's time.
    ///
    /// The export only ever holds the current week and rolls over some time
    /// after the week ends, so other weeks, an unreachable or rate-limited
    /// export, and the rollover gap all fall back to scraping. Events from
    /// the export have no `event_id`.
    ///
    /// The export is rate-limited hard: it answered HTTP 429 after about five
    /// requests in a few minutes (2026-11-08). The service fetches it once per
    /// call, so callers that call often should cache the result — the export
    /// changes rarely within a week.
    PreferThe5ers,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_the_html_scraper() {
        assert_eq!(EventSource::default(), EventSource::ForexFactory);
    }
}
