use chrono::{Datelike, NaiveDate};
use reqwest::Client;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, COOKIE, HeaderMap, HeaderValue, USER_AGENT};
use tracing::{debug, info};

use crate::error::{Error, Result};

/// Fetches the raw HTML content from Forex Factory calendar page using HTTP.
pub struct HttpCalendarFetcher {
    client: Client,
}

impl HttpCalendarFetcher {
    /// Create a new fetcher with a configured HTTP client.
    ///
    /// On native targets, automatically detects the system timezone.
    /// On WASM with the `wasm-timezone` feature, detects the browser timezone.
    /// On WASM without the feature, falls back to UTC (use `with_timezone()` to override).
    pub fn new() -> Result<Self> {
        let timezone = get_system_timezone();
        Self::with_timezone(&timezone)
    }

    /// Create a new fetcher with a specific IANA timezone.
    ///
    /// Use this on WASM to set the timezone (get it from JavaScript via
    /// `Intl.DateTimeFormat().resolvedOptions().timeZone`).
    ///
    /// # Example
    /// ```ignore
    /// // On WASM, get timezone from JS and pass it:
    /// let fetcher = HttpCalendarFetcher::with_timezone("America/New_York")?;
    /// ```
    pub fn with_timezone(timezone: &str) -> Result<Self> {
        info!("Creating HTTP client for Forex Factory...");

        let mut headers = HeaderMap::new();

        let timezone_encoded = timezone.replace('/', "%2F");
        info!("Using timezone: {timezone}");

        // Send timezone cookie so FF returns times in our local timezone
        let cookie_value = format!("fftimezone={timezone_encoded}");
        headers.insert(COOKIE, HeaderValue::from_str(&cookie_value)?);

        // Mimic a real browser
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
            ),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8"
            ),
        );
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
        headers.insert(
            "Sec-Ch-Ua",
            HeaderValue::from_static(
                "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\"",
            ),
        );
        headers.insert("Sec-Ch-Ua-Mobile", HeaderValue::from_static("?0"));
        headers.insert("Sec-Ch-Ua-Platform", HeaderValue::from_static("\"Linux\""));
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("document"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("navigate"));
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("none"));
        headers.insert("Sec-Fetch-User", HeaderValue::from_static("?1"));
        headers.insert("Upgrade-Insecure-Requests", HeaderValue::from_static("1"));

        #[cfg(not(target_arch = "wasm32"))]
        let client = {
            use std::time::Duration;
            Client::builder()
                .default_headers(headers)
                .cookie_store(true)
                .timeout(Duration::from_secs(30))
                .build()?
        };

        #[cfg(target_arch = "wasm32")]
        let client = Client::builder().default_headers(headers).build()?;

        Ok(Self { client })
    }

    /// Fetch calendar HTML for a specific week.
    pub async fn fetch_week(&self, week: &str) -> Result<String> {
        let url = format!("https://www.forexfactory.com/calendar?week={week}");
        self.fetch_url(&url).await
    }

    /// Fetch calendar HTML for a date.
    pub async fn fetch_date(&self, date: NaiveDate) -> Result<String> {
        let week = format_week_param(date);
        self.fetch_week(&week).await
    }

    /// Fetch calendar HTML for today.
    pub async fn fetch_today(&self) -> Result<String> {
        self.fetch_url("https://www.forexfactory.com/calendar?day=today")
            .await
    }

    /// Fetch calendar HTML for this week.
    pub async fn fetch_this_week(&self) -> Result<String> {
        self.fetch_url("https://www.forexfactory.com/calendar?week=this")
            .await
    }

    /// Fetch the raw HTML from a URL.
    async fn fetch_url(&self, url: &str) -> Result<String> {
        info!("Fetching calendar from: {url}");

        let response = self.client.get(url).send().await?;

        let status = response.status();
        if !status.is_success() {
            return Err(Error::HttpStatus {
                status,
                url: url.to_string(),
            });
        }

        let html = response.text().await?;

        debug!("Successfully fetched {} bytes of HTML", html.len());

        // Check if we hit Cloudflare challenge
        if html.contains("Just a moment...") || html.contains("Verifying you are human") {
            return Err(Error::CloudflareChallenge);
        }

        // Check if we got the calendar table
        if !html.contains("calendar__table") && !html.contains("calendar_row") {
            debug!("HTML preview: {}", &html[..html.len().min(500)]);
            return Err(Error::CalendarNotFound);
        }

        Ok(html)
    }
}

/// Format a date into Forex Factory's week parameter format.
fn format_week_param(date: NaiveDate) -> String {
    let month = date.format("%b").to_string().to_lowercase();
    let day = date.day();
    let year = date.year();
    format!("{month}{day}.{year}")
}

/// Get the system timezone.
///
/// - Native: uses iana-time-zone crate
/// - WASM with wasm-timezone feature: uses js-sys to query browser
/// - WASM without feature: returns "UTC"
#[cfg(not(target_arch = "wasm32"))]
fn get_system_timezone() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".to_string())
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-timezone"))]
fn get_system_timezone() -> String {
    use js_sys::{Array, Intl, Object, Reflect};

    let options = Intl::DateTimeFormat::new(&Array::new(), &Object::new()).resolved_options();

    Reflect::get(&options, &"timeZone".into())
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "UTC".to_string())
}

#[cfg(all(target_arch = "wasm32", not(feature = "wasm-timezone")))]
fn get_system_timezone() -> String {
    "UTC".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_week_param() {
        let date = NaiveDate::from_ymd_opt(2025, 6, 4).unwrap();
        assert_eq!(format_week_param(date), "jun4.2025");
    }

    #[test]
    fn test_client_creation() {
        let fetcher = HttpCalendarFetcher::new();
        assert!(fetcher.is_ok());
    }
}
