# Forex Factory Library

A Rust library for scraping economic event data from the Forex Factory calendar.

## Features

- Async HTTP client with browser mimicking
- HTML parser with pre-compiled CSS selectors
- WASM compatible
- Configurable TLS backends

## Cargo Features

```toml
[features]
default = ["rustls"]

# TLS backends - choose one
rustls = ["reqwest/rustls"]                 # Default, pure Rust
native-tls = ["reqwest/native-tls"]         # System TLS
native-tls-vendored = ["reqwest/native-tls-vendored"]  # Bundled OpenSSL
```

### Usage Examples

```bash
# Use rustls (default)
cargo add forex-factory

# Use native TLS
cargo add forex-factory --no-default-features --features native-tls

# For WASM (browser handles TLS)
cargo add forex-factory --no-default-features
```

## Structure

```
src/
├── lib.rs              # Re-exports public API
├── error.rs            # Error types (thiserror)
├── types.rs            # Module declarations for types
├── types/
│   ├── impact.rs       # Impact enum (Low, Medium, High)
│   ├── currency.rs     # Currency resolution utilities
│   ├── event.rs        # EconomicEvent struct
│   └── query.rs        # EventQuery for filtering
├── scraper.rs          # Module declarations for scraper
├── scraper/
│   ├── http_client.rs  # HttpCalendarFetcher
│   └── parser.rs       # CalendarParser
├── service.rs          # Module declarations for service
└── service/
    └── calendar.rs     # CalendarService (high-level API)
```

## Public API

- `CalendarService` - High-level async service for fetching and querying events
- `CalendarParser` - HTML parser for Forex Factory calendar pages
- `HttpCalendarFetcher` - HTTP client for fetching calendar pages
- `EconomicEvent` - Struct representing a single economic event
- `EventQuery` - Builder for filtering events
- `Impact` - Enum for event impact levels (Low, Medium, High)
- `Error` - Library error type
- `Result<T>` - Library result type alias
- `resolve_currency` - Function to resolve country names to currency codes

## Example

```rust
use forex_factory::{CalendarService, EventQuery, Impact, Result};

async fn fetch_events() -> Result<()> {
    let service = CalendarService::new()?;

    // Get all events for this week
    let events = service.get_week_events().await?;

    // Query with filters
    let query = EventQuery::new()
        .with_currency_pair("EUR/USD")
        .with_min_impact(Impact::Medium);
    let filtered = service.query_events(&query).await?;

    Ok(())
}
```

## WASM Notes

On WASM targets:
- Timezone detection is not available (browser provides timezone via cookies)
- `cookie_store` and `timeout` are handled by the browser
- TLS is handled by the browser (no feature flags needed)
