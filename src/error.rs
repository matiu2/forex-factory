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

    /// CSS selector parsing failed
    #[error("Invalid CSS selector: {0}")]
    InvalidSelector(String),
}

/// Result type alias for forex-factory operations.
pub type Result<T> = std::result::Result<T, Error>;
