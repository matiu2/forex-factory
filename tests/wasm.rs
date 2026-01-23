//! WASM-specific tests.
//!
//! Run with: `cargo test --target wasm32-unknown-unknown`
//!
//! Requires wasm-bindgen-test-runner or wasm-pack:
//! - `cargo install wasm-bindgen-cli`
//! - Or use `wasm-pack test --headless --chrome`

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use forex_factory::{CalendarService, HttpCalendarFetcher};

#[wasm_bindgen_test]
fn test_fetcher_creation() {
    // Should create without panicking (uses UTC on WASM without wasm-timezone feature)
    let fetcher = HttpCalendarFetcher::new();
    assert!(fetcher.is_ok());
}

#[wasm_bindgen_test]
fn test_fetcher_with_timezone() {
    let fetcher = HttpCalendarFetcher::with_timezone("America/New_York");
    assert!(fetcher.is_ok());
}

#[wasm_bindgen_test]
fn test_service_creation() {
    let service = CalendarService::new();
    assert!(service.is_ok());
}

#[wasm_bindgen_test]
fn test_service_with_timezone() {
    let service = CalendarService::with_timezone("Europe/London");
    assert!(service.is_ok());
}

#[cfg(feature = "wasm-timezone")]
mod wasm_timezone_tests {
    use super::*;

    #[wasm_bindgen_test]
    fn test_auto_timezone_detection() {
        // With wasm-timezone feature, this should detect the browser's timezone
        // and not fall back to UTC (unless the browser is actually in UTC)
        let service = CalendarService::new();
        assert!(service.is_ok());
        // We can't easily assert what timezone was detected, but at least
        // it shouldn't panic
    }
}
