# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-05-27

### Changed
- **BREAKING (default features):** default feature set is now
  `["native-tls", "brotli", "http2"]` (was `["rustls", "brotli"]`).
  Cloudflare's bot-fight rules now reject the `rustls` ClientHello
  fingerprint on forexfactory.com, so the library no longer works out of
  the box on `rustls`. Native TLS + HTTP/2 + Firefox-shaped headers gets
  past the challenge.
- Request headers rewritten to impersonate Firefox 150 (drops Chrome
  `Sec-Ch-Ua*` client hints, adds `Priority`). The previous Chrome
  client-hint headers were cross-checked against the TLS fingerprint by
  Cloudflare and rejected.

### Migration
- If you build with `--no-default-features`, you now need at least
  `native-tls`, `brotli`, and `http2` to talk to forexfactory.com.
- `rustls` and friends are still available as opt-in features but will
  currently 403 against forexfactory.com.

## [0.1.0] - 2025-01-23

### Added
- Initial release
- `CalendarService` for high-level async event fetching
- `CalendarService::with_timezone()` and `HttpCalendarFetcher::with_timezone()` for manual timezone setting
- `EventQuery` builder for filtering events by currency, date range, and impact
- `EconomicEvent` struct with all event data fields
- `Impact` enum (Low, Medium, High) with star ratings
- `resolve_currency` for converting country names to currency codes
- WASM compatibility with conditional compilation
- `wasm-timezone` feature for automatic browser timezone detection on WASM via `js-sys`
- Configurable TLS backends via feature flags (rustls, native-tls)
- Compression support (brotli enabled by default, gzip, deflate, zstd)
- HTTP/2, HTTP/3, SOCKS proxy, and Hickory DNS resolver features
- Browser-like headers to avoid detection
- Timezone-aware date handling

[Unreleased]: https://github.com/matiu2/forex-factory/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/matiu2/forex-factory/releases/tag/v0.1.0
