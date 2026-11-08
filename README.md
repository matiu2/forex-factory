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
default = ["native-tls", "brotli", "http2"]

# TLS backends - choose one
native-tls = ["reqwest/native-tls"]             # System TLS (default) - required to bypass Cloudflare
native-tls-vendored = ["reqwest/native-tls-vendored"]  # Bundled OpenSSL
rustls = ["reqwest/rustls"]                     # Pure Rust TLS (currently blocked by Cloudflare)
rustls-native-certs = ["reqwest/rustls-native-certs"]  # Rustls with system certs
rustls-no-provider = ["reqwest/rustls-no-provider"]    # Rustls, bring your own provider

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
# Default: native TLS + brotli + http2 (gets past Cloudflare)
cargo add forex-factory

# With extra compression support
cargo add forex-factory --features gzip

# Bundled OpenSSL (e.g. for static binaries / cross-compilation)
cargo add forex-factory --no-default-features --features native-tls-vendored,brotli,http2

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
- `EventSource` - Where events come from: scraped HTML, or The5ers' list first
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

## Matching The5ers' calendar

The5ers' economic calendar is Forex Factory's own weekly JSON export. To get
event times exactly as The5ers publish them (they count trades within ±2
minutes of a high-impact event as a breach), choose `EventSource::PreferThe5ers`:

```rust
let service = CalendarService::with_timezone("UTC")?.with_source(EventSource::PreferThe5ers);
let events = service.get_events_between(from, until).await?;
// strict, for a news rule:
let evidence = service.get_high_impact_evidence_between(from, until).await?;
```

The export only holds the current Forex Factory week — Sunday 00:00 to Sunday
00:00, New York time — and answers only a window it covers entirely. Other
windows (past weeks, a window crossing the week boundary, a seven-day
`?week=<date>` page in another timezone), and the export being unreachable,
rate-limited, or not yet rolled over, are scraped. The export is rate-limited
hard (HTTP 429 after about five requests in a few minutes): a caller that asks
often should cache `service.the5ers_week()` and pass it to
`high_impact_evidence_between(Some(&week), from, until)`.

`?week=<date>` pages are the seven days starting at `date`, and
`get_week_events_for(date)` / `get_week_high_impact_evidence_for(date)`
return exactly those days. The
two agree on timed events; they differ on untimed ones ("All Day",
"Tentative", "9th-14th"), which the export gives its own timestamp and the
scraper marks `EventTiming::Untimed` (no instant; `datetime` is the row's
midnight). Check `event.has_exact_time()` before treating `datetime` as a
release instant. Export events have no `event_id`.

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

Forex Factory is fronted by Cloudflare. The default features (`native-tls`,
`brotli`, `http2`) plus the Firefox-shaped request headers this library sends
are tuned to get past the current bot-fight rules. If you build with
`--no-default-features` you must include at least `native-tls`, `brotli`,
and `http2` or Cloudflare will return 403:

```bash
cargo add forex-factory --no-default-features --features native-tls,brotli,http2
```

The `rustls` backend's TLS ClientHello fingerprint is currently rejected by
Cloudflare on this host regardless of headers. Use `native-tls` (system
OpenSSL/Schannel/SecureTransport) or `native-tls-vendored` (bundled OpenSSL).
