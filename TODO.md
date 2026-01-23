# Forex Factory Library - Extraction TODO

Extract web scraping functionality from `../forex-factory-calendar-mcp` into this reusable library.

## Tasks

- [x] Set up Cargo.toml with required dependencies
- [x] Create types module
  - [x] impact.rs - Impact enum (Low, Medium, High)
  - [x] currency.rs - Currency resolution utilities
  - [x] event.rs - EconomicEvent struct
  - [x] query.rs - EventQuery for filtering
- [x] Create scraper module
  - [x] http_client.rs - HttpCalendarFetcher
  - [x] parser.rs - CalendarParser
- [x] Create service module
  - [x] calendar.rs - CalendarService (high-level API)
- [x] Set up lib.rs with proper re-exports
- [x] Run tests and ensure they pass (23 tests passing)
- [x] Run cargo clippy and fix issues
- [x] Run cargo fmt
- [x] Commit the changes

## Structure

```
src/
├── lib.rs              # Re-exports public API
├── types.rs            # Module declarations for types
├── types/
│   ├── impact.rs
│   ├── currency.rs
│   ├── event.rs
│   └── query.rs
├── scraper.rs          # Module declarations for scraper
├── scraper/
│   ├── http_client.rs
│   └── parser.rs
├── service.rs          # Module declarations for service
└── service/
    └── calendar.rs
```

## Public API

The library exports:

- `CalendarService` - High-level async service for fetching and querying events
- `CalendarParser` - HTML parser for Forex Factory calendar pages
- `HttpCalendarFetcher` - HTTP client for fetching calendar pages
- `EconomicEvent` - Struct representing a single economic event
- `EventQuery` - Builder for filtering events
- `Impact` - Enum for event impact levels (Low, Medium, High)
- `resolve_currency` - Function to resolve country names to currency codes
