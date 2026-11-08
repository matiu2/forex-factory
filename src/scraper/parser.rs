use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use scraper::{Html, Selector};
use tracing::debug;

use crate::error::{Error, Result};
use crate::types::{EconomicEvent, EventTiming, Impact};
mod evidence;
mod row_time;
use row_time::RowTime;

/// Parses Forex Factory calendar HTML into structured events.
pub struct CalendarParser {
    timezone: Option<chrono_tz::Tz>,
    // Selectors are compiled once and reused
    row_selector: Selector,
    date_selector: Selector,
    currency_selector: Selector,
    impact_selector: Selector,
    event_selector: Selector,
    time_selector: Selector,
    actual_selector: Selector,
    forecast_selector: Selector,
    previous_selector: Selector,
}

impl CalendarParser {
    /// Create a new parser with pre-compiled CSS selectors.
    pub fn new() -> Result<Self> {
        Ok(Self {
            timezone: None,
            // Use data-event-id attribute to find actual event rows
            row_selector: Selector::parse("tr[data-event-id]")
                .map_err(|e| Error::InvalidSelector(format!("{e:?}")))?,
            date_selector: Selector::parse("td.calendar__date")
                .map_err(|e| Error::InvalidSelector(format!("{e:?}")))?,
            currency_selector: Selector::parse("td.calendar__currency")
                .map_err(|e| Error::InvalidSelector(format!("{e:?}")))?,
            // Impact icon is in a span with class like "icon--ff-impact-yel"
            impact_selector: Selector::parse("td.calendar__impact span")
                .map_err(|e| Error::InvalidSelector(format!("{e:?}")))?,
            event_selector: Selector::parse("td.calendar__event span.calendar__event-title")
                .map_err(|e| Error::InvalidSelector(format!("{e:?}")))?,
            time_selector: Selector::parse("td.calendar__time")
                .map_err(|e| Error::InvalidSelector(format!("{e:?}")))?,
            actual_selector: Selector::parse("td.calendar__actual")
                .map_err(|e| Error::InvalidSelector(format!("{e:?}")))?,
            forecast_selector: Selector::parse("td.calendar__forecast")
                .map_err(|e| Error::InvalidSelector(format!("{e:?}")))?,
            previous_selector: Selector::parse("td.calendar__previous")
                .map_err(|e| Error::InvalidSelector(format!("{e:?}")))?,
        })
    }

    /// Parse wall-clock times in the same IANA timezone requested from the source.
    /// Returned events keep the existing Local timestamp representation while
    /// preserving the correct absolute instant.
    pub fn with_timezone(timezone: &str) -> Result<Self> {
        let mut parser = Self::new()?;
        parser.timezone = Some(
            timezone
                .parse()
                .map_err(|_| Error::InvalidTimezone(timezone.into()))?,
        );
        Ok(parser)
    }

    /// 00:00 on `date` in the timezone this parser reads times in (the
    /// host's when none was given) — where a `?week=<date>` page starts.
    pub(crate) fn midnight(&self, date: NaiveDate) -> Result<chrono::DateTime<chrono::Utc>> {
        let naive = date.and_time(NaiveTime::MIN);
        let at = match self.timezone {
            Some(timezone) => timezone
                .from_local_datetime(&naive)
                .earliest()
                .map(|at| at.with_timezone(&chrono::Utc)),
            None => Local
                .from_local_datetime(&naive)
                .earliest()
                .map(|at| at.with_timezone(&chrono::Utc)),
        };
        at.ok_or_else(|| Error::InvalidEventTime(format!("{naive} has no midnight")))
    }

    /// Best-effort legacy parsing. Use parse_checked when completeness is required.
    /// Parse HTML content into a list of economic events.
    /// The `base_date` is used as fallback and to determine the year for date parsing.
    pub fn parse(&self, html: &str, base_date: NaiveDate) -> Vec<EconomicEvent> {
        match self.parse_internal(html, base_date, false, false) {
            Ok(events) => events,
            Err(error) => {
                tracing::error!(%error,"legacy calendar parse failed");
                Vec::new()
            }
        }
    }

    /// Parse complete calendar evidence. Ambiguous or nonexistent explicit-zone
    /// event times return an error, so consumers cannot approve an incomplete
    /// calendar after silently losing a related release.
    pub fn parse_checked(&self, html: &str, base_date: NaiveDate) -> Result<Vec<EconomicEvent>> {
        self.parse_internal(html, base_date, true, false)
    }

    /// Complete evidence for high-impact entry restrictions. Unknown fields and
    /// missing or unrecognized timing fail rather than becoming low-impact or
    /// midnight events. Rows with no release time in a known form ("All Day",
    /// "Tentative", "Day 2", "9th-14th", "Oct 17th") are kept, marked
    /// [`EventTiming::Untimed`], at any impact. Recognized non-economic
    /// (holiday) rows are skipped.
    pub fn parse_high_impact_evidence(
        &self,
        html: &str,
        base_date: NaiveDate,
    ) -> Result<Vec<EconomicEvent>> {
        if self.timezone.is_none() {
            return Err(Error::InvalidCalendarEvidence(
                "explicit source timezone is required".into(),
            ));
        }
        self.parse_internal(html, base_date, true, true)
    }

    fn parse_internal(
        &self,
        html: &str,
        base_date: NaiveDate,
        checked: bool,
        evidence: bool,
    ) -> Result<Vec<EconomicEvent>> {
        debug!("Parsing HTML of {} bytes for date {base_date}", html.len());
        let document = Html::parse_document(html);
        let mut events = Vec::new();
        let mut current_date = base_date;
        let mut current_time: Option<RowTime> = None;
        let mut has_date = false;

        let row_count = document.select(&self.row_selector).count();
        debug!("Found {row_count} event rows in HTML");

        if evidence && row_count == 0 {
            return Err(Error::CalendarNotFound);
        }
        for row in document.select(&self.row_selector) {
            if !self.extract_text(&row, &self.date_selector).is_empty() {
                has_date = true;
            }
            if evidence && !has_date {
                return Err(Error::InvalidCalendarEvidence(
                    "first event has no explicit date".into(),
                ));
            }
            let event = self.parse_row(
                &row,
                &mut current_date,
                &mut current_time,
                base_date,
                evidence,
            );

            match event {
                Ok(Some(e)) => {
                    debug!("Parsed event: {} ({}) - {}", e.name, e.currency, e.impact);
                    events.push(e);
                }
                Ok(None) => {
                    // Row had no parseable event (e.g., header row)
                    continue;
                }
                Err(error) if checked => return Err(error),
                Err(error) => {
                    tracing::warn!(%error,"legacy calendar parser skipped an invalid event")
                }
            }
        }

        Ok(events)
    }

    /// Parse a single table row into an event.
    /// Returns None if the row doesn't contain event data.
    fn parse_row(
        &self,
        row: &scraper::ElementRef,
        current_date: &mut NaiveDate,
        current_time: &mut Option<RowTime>,
        reference: NaiveDate,
        evidence: bool,
    ) -> Result<Option<EconomicEvent>> {
        // Update date if present in this row
        let date_text = self.extract_text(row, &self.date_selector);
        let date = if evidence {
            evidence::date(&date_text, reference)
        } else {
            parse_date(&date_text, reference.year())
        };
        if evidence && !date_text.is_empty() && date.is_none() {
            return Err(Error::InvalidCalendarEvidence(
                "unrecognized explicit event date".into(),
            ));
        }
        if let Some(parsed_date) = date {
            debug!("Parsed date from row: {parsed_date}");
            *current_date = parsed_date;
            // Reset time when date changes - events on new day start fresh
            *current_time = None;
        }

        let currency = self.extract_text(row, &self.currency_selector);
        let currency = if evidence && currency.eq_ignore_ascii_case("All") {
            // Forex Factory global events apply to every currency. Preserve the
            // sentinel so a consumer cannot discard a timed high-impact release.
            "All".to_owned()
        } else if evidence {
            currency
                .parse::<crate::types::Currency>()
                .map_err(|_| {
                    Error::InvalidCalendarEvidence("missing or unknown event currency".into())
                })?
                .code()
                .to_owned()
        } else {
            currency
        };
        if currency.is_empty() {
            return if evidence {
                Err(Error::InvalidCalendarEvidence(
                    "event currency is missing".into(),
                ))
            } else {
                Ok(None)
            };
        }

        // Read on every row, holidays included, so a blank cell below any row
        // shares that row's time.
        let time_text = self.extract_text(row, &self.time_selector);
        let cell = row_time::classify(&time_text);
        let row_time = row_time::resolve(&cell, current_time);
        let impact = if evidence {
            let Some(impact) = evidence::impact(self, row)? else {
                return Ok(None);
            };
            impact
        } else {
            self.extract_impact(row).unwrap_or(Impact::Low)
        };
        let name = self.extract_text(row, &self.event_selector);
        if name.is_empty() {
            return if evidence {
                Err(Error::InvalidCalendarEvidence(
                    "event name is missing".into(),
                ))
            } else {
                Ok(None)
            };
        }

        if evidence {
            evidence::accepts(row_time.as_ref(), impact)?;
        }
        // An untimed row sits at midnight of its own day, marked untimed —
        // never at the time of a row above it.
        let (time, timing) = match row_time {
            Some(RowTime::Clock(time)) => (time, EventTiming::Exact),
            Some(RowTime::Untimed(label)) => (NaiveTime::MIN, EventTiming::Untimed { label }),
            None => (
                NaiveTime::MIN,
                EventTiming::Untimed {
                    label: String::new(),
                },
            ),
        };
        let datetime = NaiveDateTime::new(*current_date, time);
        let datetime_local = match self.timezone {
            Some(timezone) => timezone
                .from_local_datetime(&datetime)
                .single()
                .ok_or_else(|| Error::InvalidEventTime(format!("{datetime} in {timezone}")))?
                .with_timezone(&Local),
            None => Local
                .from_local_datetime(&datetime)
                .single()
                .unwrap_or_else(|| Local.from_utc_datetime(&datetime)),
        };

        let actual = self.extract_text(row, &self.actual_selector);
        let forecast = self.extract_text(row, &self.forecast_selector);
        let previous = self.extract_text(row, &self.previous_selector);

        // The row's own id. Already required by the row SELECTOR
        // (`tr[data-event-id]`) and, until now, discarded — see
        // `EconomicEvent::event_id` for what it is good for. Empty is
        // normalised to None so a caller never has to distinguish "absent"
        // from "present but blank".
        let event_id = row
            .value()
            .attr("data-event-id")
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_owned);

        Ok(Some(EconomicEvent {
            name,
            currency,
            impact,
            datetime: datetime_local,
            timing,
            event_id,
            actual: if actual.is_empty() {
                None
            } else {
                Some(actual)
            },
            forecast: if forecast.is_empty() {
                None
            } else {
                Some(forecast)
            },
            previous: if previous.is_empty() {
                None
            } else {
                Some(previous)
            },
        }))
    }

    /// Extract text content from the first matching element.
    fn extract_text(&self, row: &scraper::ElementRef, selector: &Selector) -> String {
        row.select(selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default()
    }

    /// Extract impact level from the impact cell's span class.
    fn extract_impact(&self, row: &scraper::ElementRef) -> Option<Impact> {
        row.select(&self.impact_selector)
            .next()
            .and_then(|el| el.value().attr("class").and_then(Impact::from_ff_class))
    }
}

/// Parse date string from Forex Factory format.
/// Examples: "Tue Jan 13", "Mon Jan 12", "Wed Feb 5"
/// The `reference_year` is needed because FF doesn't include year in dates.
fn parse_date(date_str: &str, reference_year: i32) -> Option<NaiveDate> {
    let date_str = date_str.trim();
    if date_str.is_empty() {
        return None;
    }

    // FF format: "Tue Jan 13" or "TueJan 13" (sometimes no space after day name)
    // We need to extract month and day
    let parts: Vec<&str> = date_str.split_whitespace().collect();

    // Expected: ["Tue", "Jan", "13"] or ["TueJan", "13"]
    let (month_str, day_str) = match parts.len() {
        3 => (parts[1], parts[2]),
        2 => {
            // Day name might be concatenated with month: "TueJan 13"
            let first = parts[0];
            if first.len() >= 6 {
                (first.get(3..)?, parts[1])
            } else {
                return None;
            }
        }
        _ => return None,
    };

    let month = match month_str.to_lowercase().as_str() {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => return None,
    };

    let day: u32 = day_str.parse().ok()?;

    NaiveDate::from_ymd_opt(reference_year, month, day)
}

/// Parse time string from Forex Factory format.
/// Examples: "8:30am", "2:00pm", "12:30am"
fn parse_time(time_str: &str) -> Option<NaiveTime> {
    let time_str = time_str.trim().to_lowercase();

    // Try various formats
    if let Ok(time) = NaiveTime::parse_from_str(&time_str, "%l:%M%P") {
        return Some(time);
    }
    if let Ok(time) = NaiveTime::parse_from_str(&time_str, "%I:%M%P") {
        return Some(time);
    }
    if let Ok(time) = NaiveTime::parse_from_str(&time_str, "%H:%M") {
        return Some(time);
    }

    None
}

impl Default for CalendarParser {
    fn default() -> Self {
        Self::new().expect("Default selectors should be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time() {
        // Note: These tests verify the parse_time function handles various formats
        // The actual parsing may need adjustment based on real Forex Factory data

        // 24-hour format
        assert_eq!(
            parse_time("14:00"),
            Some(NaiveTime::from_hms_opt(14, 0, 0).unwrap())
        );
    }

    #[test]
    fn test_parser_creation() {
        let parser = CalendarParser::new();
        assert!(parser.is_ok());
    }

    /// One real calendar row, copied from a live `?week=this` fetch rather
    /// than hand-written, so the selectors are exercised against the markup
    /// Forex Factory actually serves — including the `calendar__event-title`
    /// nested two spans deep, which a simplified fixture would not reproduce.
    const REAL_ROW: &str = r#"<table><tbody>
      <tr data-event-id="150129" data-day-dateline="1789826400" class="calendar__row">
        <td class="calendar__cell calendar__date"><span class="date">Sun <span>Sep 20</span></span></td>
        <td class="calendar__cell calendar__time">11:00am</td>
        <td class="calendar__cell calendar__currency">CNY</td>
        <td class="calendar__cell calendar__impact"><span class="icon icon--ff-impact-yel"></span></td>
        <td class="calendar__cell calendar__event">
          <div class="calendar__event-wrapper">
            <span class="calendar__event-title-wrapper fadeout-end">
              <span class="calendar__event-title">1-y Loan Prime Rate</span>
            </span>
          </div>
        </td>
        <td class="calendar__cell calendar__actual">3.00%</td>
        <td class="calendar__cell calendar__forecast">3.00%</td>
        <td class="calendar__cell calendar__previous">3.00%</td>
      </tr>
    </tbody></table>"#;

    /// The event id is CAPTURED, not merely used to select the row.
    ///
    /// `tr[data-event-id]` was already the row selector, so the attribute has
    /// always been present and its value was simply thrown away. It is the
    /// only stable per-event handle Forex Factory exposes — there is no anchor
    /// on the row (the detail view is JS-driven) — and
    /// `forexfactory.com/calendar?event=<id>` redirects to the right day and
    /// opens that event's detail panel. Verified live before this was written.
    #[test]
    fn parses_the_event_id() {
        let parser = CalendarParser::new().expect("parser");
        let base = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
        let events = parser.parse(REAL_ROW, base);

        assert_eq!(events.len(), 1, "one row should parse to one event");
        assert_eq!(
            events[0].event_id.as_deref(),
            Some("150129"),
            "the id in `data-event-id` must reach the event"
        );
        // ...and the rest of the row still parses, so capturing the id did not
        // disturb the existing extraction.
        assert_eq!(events[0].name, "1-y Loan Prime Rate");
        assert_eq!(events[0].currency, "CNY");
    }

    /// A row WITHOUT the attribute must still parse, with `event_id: None`.
    ///
    /// The field is `Option` precisely so a markup change upstream degrades to
    /// "no deep link for this event" rather than dropping the event — losing a
    /// 3-star print because its id moved would be far worse than losing a link.
    #[test]
    fn a_row_without_an_id_still_parses() {
        // NOTE: the row selector itself requires `data-event-id`, so this
        // asserts the OPTION handling in the constructor rather than a row the
        // parser would see today. It is the guard that matters if the selector
        // is ever widened.
        let parser = CalendarParser::new().expect("parser");
        let base = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
        let html = REAL_ROW.replace(r#"data-event-id="150129""#, r#"data-event-id="""#);
        let events = parser.parse(&html, base);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].event_id, None,
            "an empty id must be None, never Some(\"\")"
        );
    }

    #[test]
    fn test_parse_date() {
        // Standard format: "Tue Jan 13"
        assert_eq!(
            parse_date("Tue Jan 13", 2026),
            Some(NaiveDate::from_ymd_opt(2026, 1, 13).unwrap())
        );

        // Different month
        assert_eq!(
            parse_date("Mon Feb 3", 2026),
            Some(NaiveDate::from_ymd_opt(2026, 2, 3).unwrap())
        );

        // Empty string returns None
        assert_eq!(parse_date("", 2026), None);

        // Whitespace only returns None
        assert_eq!(parse_date("   ", 2026), None);
    }
}
