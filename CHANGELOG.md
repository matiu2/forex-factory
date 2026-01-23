# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `wasm-timezone` feature for automatic browser timezone detection on WASM via `js-sys`
- `CalendarService::with_timezone()` and `HttpCalendarFetcher::with_timezone()` for manual timezone setting

## [0.1.0] - 2025-01-23

### Added
- Initial release
- `CalendarService` for high-level async event fetching
- `EventQuery` builder for filtering events by currency, date range, and impact
- `EconomicEvent` struct with all event data fields
- `Impact` enum (Low, Medium, High) with star ratings
- `resolve_currency` for converting country names to currency codes
- WASM compatibility with conditional compilation
- Configurable TLS backends via feature flags (rustls, native-tls)
- Compression support (brotli enabled by default, gzip, deflate, zstd)
- HTTP/2, HTTP/3, SOCKS proxy, and Hickory DNS resolver features
- Browser-like headers to avoid detection
- Timezone-aware date handling

[Unreleased]: https://github.com/matiu2/forex-factory/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/matiu2/forex-factory/releases/tag/v0.1.0
