use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use scraper::{Html, Selector};
use tracing::debug;

use crate::error::{Error, Result};
use crate::types::{EconomicEvent, Impact};

/// Parses Forex Factory calendar HTML into structured events.
pub struct CalendarParser {
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

    /// Parse HTML content into a list of economic events.
    /// The `base_date` is used as fallback and to determine the year for date parsing.
    pub fn parse(&self, html: &str, base_date: NaiveDate) -> Vec<EconomicEvent> {
        debug!("Parsing HTML of {} bytes for date {base_date}", html.len());
        let document = Html::parse_document(html);
        let mut events = Vec::new();
        let mut current_date = base_date;
        let mut current_time: Option<NaiveTime> = None;
        let reference_year = base_date.year();

        let row_count = document.select(&self.row_selector).count();
        debug!("Found {row_count} event rows in HTML");

        for row in document.select(&self.row_selector) {
            let event = self.parse_row(&row, &mut current_date, &mut current_time, reference_year);

            match event {
                Some(e) => {
                    debug!("Parsed event: {} ({}) - {}", e.name, e.currency, e.impact);
                    events.push(e);
                }
                None => {
                    // Row had no parseable event (e.g., header row)
                    continue;
                }
            }
        }

        events
    }

    /// Parse a single table row into an event.
    /// Returns None if the row doesn't contain event data.
    fn parse_row(
        &self,
        row: &scraper::ElementRef,
        current_date: &mut NaiveDate,
        current_time: &mut Option<NaiveTime>,
        reference_year: i32,
    ) -> Option<EconomicEvent> {
        // Update date if present in this row
        let date_text = self.extract_text(row, &self.date_selector);
        if let Some(parsed_date) = parse_date(&date_text, reference_year) {
            debug!("Parsed date from row: {parsed_date}");
            *current_date = parsed_date;
            // Reset time when date changes - events on new day start fresh
            *current_time = None;
        }

        let currency = self.extract_text(row, &self.currency_selector);
        if currency.is_empty() {
            return None;
        }

        let impact = self.extract_impact(row).unwrap_or(Impact::Low);
        let name = self.extract_text(row, &self.event_selector);
        if name.is_empty() {
            return None;
        }

        // Update time if present in this row
        let time_text = self.extract_text(row, &self.time_selector);
        if !time_text.is_empty()
            && time_text != "All Day"
            && time_text != "Tentative"
            && let Some(parsed_time) = parse_time(&time_text)
        {
            *current_time = Some(parsed_time);
        }

        let time = current_time.unwrap_or(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        let datetime = NaiveDateTime::new(*current_date, time);
        // Forex Factory times are shown in user's local timezone
        let datetime_local = Local
            .from_local_datetime(&datetime)
            .single()
            .unwrap_or_else(|| Local.from_utc_datetime(&datetime));

        let actual = self.extract_text(row, &self.actual_selector);
        let forecast = self.extract_text(row, &self.forecast_selector);
        let previous = self.extract_text(row, &self.previous_selector);

        Some(EconomicEvent {
            name,
            currency,
            impact,
            datetime: datetime_local,
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
        })
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
                (&first[3..], parts[1])
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
