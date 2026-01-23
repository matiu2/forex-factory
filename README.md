# Forex Factory Library

A Rust library for scraping economic event data from the Forex Factory calendar.

## Features

- Async HTTP client with browser mimicking
- HTML parser with pre-compiled CSS selectors
- WASM compatible
- Configurable TLS backends, compression, and more

## Cargo Features

```toml
[features]
default = ["rustls", "brotli"]

# TLS backends - choose one
rustls = ["reqwest/rustls"]                     # Pure Rust TLS (default)
rustls-native-certs = ["reqwest/rustls-native-certs"]  # Rustls with system certs
rustls-no-provider = ["reqwest/rustls-no-provider"]    # Rustls, bring your own provider
native-tls = ["reqwest/native-tls"]             # System TLS - better Cloudflare compatibility
native-tls-vendored = ["reqwest/native-tls-vendored"]  # Bundled OpenSSL

# Compression
gzip = ["reqwest/gzip"]
brotli = ["reqwest/brotli"]
deflate = ["reqwest/deflate"]
zstd = ["reqwest/zstd"]

# Other
http2 = ["reqwest/http2"]       # HTTP/2 support
http3 = ["reqwest/http3"]       # HTTP/3 support (unstable)
socks = ["reqwest/socks"]       # SOCKS5 proxy support
hickory-dns = ["reqwest/hickory-dns"]  # Hickory DNS resolver

# WASM
wasm-timezone = ["js-sys"]      # Auto-detect browser timezone on WASM
```

### Usage Examples

```bash
# Use rustls (default)
cargo add forex-factory

# Use native TLS (recommended if you encounter Cloudflare challenges)
cargo add forex-factory --no-default-features --features native-tls,brotli

# With compression support
cargo add forex-factory --features gzip,brotli

# For WASM (browser handles TLS)
cargo add forex-factory --no-default-features

# For WASM with automatic timezone detection
cargo add forex-factory --no-default-features --features wasm-timezone
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
- `cookie_store` and `timeout` are handled by the browser
- TLS is handled by the browser (no TLS feature flags needed)
- Use `--no-default-features` to avoid pulling in native TLS dependencies

### Timezone on WASM

On native targets, the library automatically detects your system timezone.

On WASM, you have two options:

**Option 1: Enable `wasm-timezone` feature (recommended)**

```bash
cargo add forex-factory --no-default-features --features wasm-timezone
```

This uses `js-sys` to automatically detect the browser's timezone via
`Intl.DateTimeFormat().resolvedOptions().timeZone`.

**Option 2: Manual timezone**

```rust
// Get timezone from JS yourself and pass it:
let service = CalendarService::with_timezone("America/New_York")?;
```

Without either option, WASM defaults to UTC.

## Troubleshooting

### Cloudflare Challenges

Forex Factory uses Cloudflare protection. If you encounter `CloudflareChallenge` errors,
try switching to `native-tls`:

```bash
cargo add forex-factory --no-default-features --features native-tls,brotli
```

The `native-tls` backend uses your system's TLS implementation (OpenSSL on Linux,
Schannel on Windows, SecureTransport on macOS), which has better TLS fingerprinting
compatibility with Cloudflare than the pure-Rust `rustls` backend.
