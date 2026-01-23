use chrono::{Datelike, NaiveDate};
use color_eyre::{Result, eyre::eyre};
use reqwest::Client;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, COOKIE, HeaderMap, HeaderValue, USER_AGENT};
use std::time::Duration;
use tracing::{debug, info};

/// Fetches the raw HTML content from Forex Factory calendar page using HTTP.
pub struct HttpCalendarFetcher {
    client: Client,
}

impl HttpCalendarFetcher {
    /// Create a new fetcher with a configured HTTP client.
    pub fn new() -> Result<Self> {
        info!("Creating HTTP client for Forex Factory...");

        // Get system timezone to send to Forex Factory
        let timezone = iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".to_string());
        let timezone_encoded = timezone.replace('/', "%2F");
        info!("Using timezone: {timezone}");

        let mut headers = HeaderMap::new();

        // Send timezone cookie so FF returns times in our local timezone
        let cookie_value = format!("fftimezone={timezone_encoded}");
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&cookie_value).map_err(|e| eyre!("Invalid cookie value: {e}"))?,
        );

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

        let client = Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| eyre!("Failed to build HTTP client: {e}"))?;

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

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| eyre!("Failed to fetch {url}: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(eyre!("HTTP error {status} for {url}"));
        }

        let html = response
            .text()
            .await
            .map_err(|e| eyre!("Failed to read response body: {e}"))?;

        debug!("Successfully fetched {} bytes of HTML", html.len());

        // Check if we hit Cloudflare challenge
        if html.contains("Just a moment...") || html.contains("Verifying you are human") {
            return Err(eyre!(
                "Cloudflare challenge detected. The site requires browser verification."
            ));
        }

        // Check if we got the calendar table
        if !html.contains("calendar__table") && !html.contains("calendar_row") {
            debug!("HTML preview: {}", &html[..html.len().min(500)]);
            return Err(eyre!(
                "Calendar table not found in response. Page structure may have changed."
            ));
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
