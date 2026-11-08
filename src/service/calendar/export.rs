//! The5ers' export used for exactly the windows it covers, scraping for the
//! rest.
use chrono::{DateTime, NaiveDate, Utc};

use super::CalendarService;
use crate::error::{Error, Result};
use crate::scraper::The5ersWeek;
use crate::types::EconomicEvent;

impl CalendarService {
    /// Fetch The5ers' export (Forex Factory's weekly JSON) whatever the
    /// service's source. `Ok(None)` when it is empty. For callers that cache
    /// it to stay under its rate limit and pass it to
    /// [`high_impact_evidence_between`](Self::high_impact_evidence_between).
    pub async fn the5ers_week(&self) -> Result<Option<The5ersWeek>> {
        let json = self.fetcher.fetch_feed().await?;
        The5ersWeek::parse(&json)
    }

    /// Strict high-impact evidence for `[from, until)`. With
    /// [`EventSource::PreferThe5ers`](crate::EventSource) the export answers
    /// when it covers the whole window; otherwise the window's pages are
    /// scraped, each one strictly.
    pub async fn get_high_impact_evidence_between(
        &self,
        from: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<Vec<EconomicEvent>> {
        let week = self.load_feed().await;
        self.high_impact_evidence_between(week.as_ref(), from, until)
            .await
    }

    /// [`get_high_impact_evidence_between`](Self::get_high_impact_evidence_between)
    /// with an export the caller already holds: `week` answers when it covers
    /// `[from, until)`, and the service's own source is not consulted.
    pub async fn high_impact_evidence_between(
        &self,
        week: Option<&The5ersWeek>,
        from: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<Vec<EconomicEvent>> {
        self.between(week, from, until, true).await
    }

    /// Display events for `[from, until)`, from the export when the source
    /// wants it and it covers the whole window, scraped otherwise.
    pub async fn get_events_between(
        &self,
        from: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<Vec<EconomicEvent>> {
        let week = self.load_feed().await;
        self.between(week.as_ref(), from, until, false).await
    }

    async fn between(
        &self,
        week: Option<&The5ersWeek>,
        from: DateTime<Utc>,
        until: DateTime<Utc>,
        evidence: bool,
    ) -> Result<Vec<EconomicEvent>> {
        if from > until {
            return Err(Error::InvalidCalendarEvidence(format!(
                "inverted window {from} .. {until}"
            )));
        }
        if let Some(week) = week.filter(|week| week.covers(from, until)) {
            return Ok(week.events_between(from, until));
        }
        if let Some(week) = week {
            tracing::info!(%from, %until, start = %week.start(), end = %week.end(),
                "The5ers export does not cover the whole window; scraping");
        }
        let mut events = Vec::new();
        let mut page = self.page_containing(from)?;
        while self.parser.midnight(page)? < until {
            let scraped = self.scrape_page(page, evidence).await?;
            events.extend(
                scraped
                    .into_iter()
                    .filter(|e| (from..until).contains(&e.datetime.with_timezone(&Utc))),
            );
            page = page + chrono::Days::new(7);
        }
        Ok(events)
    }

    /// The date whose page starts at or before `at`: `at`'s own day in the
    /// calendar timezone.
    fn page_containing(&self, at: DateTime<Utc>) -> Result<NaiveDate> {
        let day = at.date_naive();
        // `at`'s UTC day, stepped back until its midnight in the calendar
        // timezone is not after `at` (at most one step for any real zone).
        [day, day.pred_opt().unwrap_or(day)]
            .into_iter()
            .map(|day| self.parser.midnight(day).map(|start| (day, start)))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|(_, start)| *start <= at)
            .map(|(day, _)| day)
            .max()
            .ok_or_else(|| Error::InvalidEventTime(format!("no calendar day contains {at}")))
    }

    /// `days` days of `feed` from `date`'s midnight, when it covers them all.
    pub(super) fn covered_days(
        &self,
        feed: Option<&The5ersWeek>,
        date: NaiveDate,
        days: u64,
    ) -> Result<Option<Vec<EconomicEvent>>> {
        let Some(feed) = feed else {
            return Ok(None);
        };
        let from = self.parser.midnight(date)?;
        let until = self.parser.midnight(date + chrono::Days::new(days))?;
        if feed.covers(from, until) {
            return Ok(Some(feed.events_between(from, until)));
        }
        tracing::info!("The5ers export does not cover {days} days from {date}; scraping");
        Ok(None)
    }

    /// The `?week=<date>` page: the seven days starting at `date`.
    pub(super) async fn scrape_page(
        &self,
        date: NaiveDate,
        evidence: bool,
    ) -> Result<Vec<EconomicEvent>> {
        let html = self.fetcher.fetch_date(date).await?;
        if evidence {
            self.parser.parse_high_impact_evidence(&html, date)
        } else {
            self.parser.parse_checked(&html, date)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// The real export for the week of 2026-11-08, fetched once on
    /// 2026-11-08 (Sunday 8 00:00 → Sunday 15 00:00 New York, EST).
    const WEEK: &str = include_str!("../../../tests/fixtures/ff_calendar_week_2026-11-08.json");

    fn week() -> The5ersWeek {
        The5ersWeek::parse(WEEK).unwrap().unwrap()
    }

    fn utc(d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, d, h, 0, 0).unwrap()
    }

    fn service() -> CalendarService {
        CalendarService::with_timezone("UTC").unwrap()
    }

    #[test]
    fn the_export_spans_new_york_sunday_to_sunday() {
        assert_eq!(week().start(), utc(8, 5));
        assert_eq!(week().end(), utc(15, 5));
    }

    /// A live window (the worker asks for now-2min .. now+1 day) inside the
    /// export's week is answered by the export alone.
    #[tokio::test]
    async fn a_window_inside_the_week_is_answered_by_the_export() {
        let week = week();
        let events = service()
            .high_impact_evidence_between(Some(&week), utc(10, 0), utc(11, 0))
            .await
            .unwrap();
        let cpi = events.iter().find(|e| e.name == "CPI m/m").unwrap();
        assert_eq!(
            cpi.datetime.with_timezone(&Utc),
            utc(10, 13) + chrono::Duration::minutes(30)
        );
        assert!(
            events
                .iter()
                .all(|e| e.datetime.with_timezone(&Utc) < utc(11, 0))
        );
    }

    /// A seven-day page in UTC never lines up with the New York week, so the
    /// export does not answer for it.
    #[test]
    fn a_utc_page_is_not_covered() {
        let week = week();
        for day in [8, 9] {
            let date = NaiveDate::from_ymd_opt(2026, 11, day).unwrap();
            assert_eq!(service().covered_days(Some(&week), date, 7).unwrap(), None);
        }
        let monday = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let day = service()
            .covered_days(Some(&week), monday, 1)
            .unwrap()
            .unwrap();
        assert!(day.iter().any(|e| e.name == "Leading Indicators"));
    }

    #[test]
    fn a_window_starts_on_its_own_days_page() {
        let utc_service = service();
        assert_eq!(
            utc_service.page_containing(utc(9, 13)).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
        );
        // 02:00 UTC Monday is still Sunday evening in New York.
        let new_york = CalendarService::with_timezone("America/New_York").unwrap();
        assert_eq!(
            new_york.page_containing(utc(9, 2)).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11, 8).unwrap()
        );
    }

    #[tokio::test]
    async fn an_inverted_window_is_an_error() {
        assert!(
            service()
                .high_impact_evidence_between(None, utc(10, 0), utc(9, 0))
                .await
                .is_err()
        );
    }
}
