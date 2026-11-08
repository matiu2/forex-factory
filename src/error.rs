//! Error types for the forex-factory library.

use thiserror::Error;

/// Errors that can occur when using the forex-factory library.
#[derive(Debug, Error)]
pub enum Error {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Invalid HTTP header value
    #[error("Invalid header value: {0}")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),

    /// HTTP error response (non-2xx status)
    #[error("HTTP error {status} for {url}")]
    HttpStatus {
        status: reqwest::StatusCode,
        url: String,
    },

    /// Cloudflare challenge detected
    #[error("Cloudflare challenge detected. The site requires browser verification.")]
    CloudflareChallenge,

    /// Calendar table not found in response
    #[error("Calendar table not found in response. Page structure may have changed.")]
    CalendarNotFound,

    /// The requested calendar timezone is not a known IANA timezone.
    #[error("Invalid calendar timezone: {0}")]
    InvalidTimezone(String),

    /// A calendar wall time cannot identify one instant in its requested zone.
    #[error("Invalid calendar event time: {0}")]
    InvalidEventTime(String),

    /// A source row cannot establish complete high-impact calendar evidence.
    #[error("Incomplete calendar evidence: {0}")]
    InvalidCalendarEvidence(String),

    /// The weekly JSON export was not the expected JSON (when rate-limited it
    /// answers with an HTML page instead)
    #[error("Invalid calendar export JSON: {0}")]
    FeedJson(#[from] serde_json::Error),

    /// CSS selector parsing failed
    #[error("Invalid CSS selector: {0}")]
    InvalidSelector(String),
}

/// Result type alias for forex-factory operations.
pub type Result<T> = std::result::Result<T, Error>;
