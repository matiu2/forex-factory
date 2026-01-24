//! Currency types and resolution from country names and abbreviations.
//!
//! Provides a `Currency` enum for major forex currencies, plus a flexible
//! `resolve_currency` function for string-based lookups.

use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Major currencies tracked by Forex Factory.
///
/// Use `FromStr` to parse from strings (case-insensitive):
/// ```
/// use forex_factory::Currency;
///
/// let usd: Currency = "USD".parse().unwrap();
/// let eur: Currency = "eur".parse().unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, FromStr, Serialize, Deserialize)]
#[display(style = "UPPERCASE")]
#[from_str(regex = "(?i)(?<>)")]
pub enum Currency {
    /// US Dollar
    USD,
    /// Euro
    EUR,
    /// British Pound
    GBP,
    /// Japanese Yen
    JPY,
    /// Australian Dollar
    AUD,
    /// Canadian Dollar
    CAD,
    /// Swiss Franc
    CHF,
    /// New Zealand Dollar
    NZD,
    /// Chinese Yuan
    #[from_str(regex = "(?i)CN[YH]")]
    CNY,
}

impl Currency {
    /// Get the ISO 4217 code as a string
    pub fn code(&self) -> &'static str {
        match self {
            Currency::USD => "USD",
            Currency::EUR => "EUR",
            Currency::GBP => "GBP",
            Currency::JPY => "JPY",
            Currency::AUD => "AUD",
            Currency::CAD => "CAD",
            Currency::CHF => "CHF",
            Currency::NZD => "NZD",
            Currency::CNY => "CNY",
        }
    }

    /// Get the full name of the currency
    pub fn name(&self) -> &'static str {
        match self {
            Currency::USD => "US Dollar",
            Currency::EUR => "Euro",
            Currency::GBP => "British Pound",
            Currency::JPY => "Japanese Yen",
            Currency::AUD => "Australian Dollar",
            Currency::CAD => "Canadian Dollar",
            Currency::CHF => "Swiss Franc",
            Currency::NZD => "New Zealand Dollar",
            Currency::CNY => "Chinese Yuan",
        }
    }

    /// Parse from country name, demonym, or code
    ///
    /// More flexible than `FromStr`, handles aliases like "Canada" → CAD
    pub fn from_alias(input: &str) -> Option<Self> {
        resolve_currency_enum(input)
    }
}

/// Static mapping of country names/variations to Currency enum.
/// Keys are lowercase for case-insensitive lookup.
static ALIAS_TO_CURRENCY: LazyLock<HashMap<&'static str, Currency>> = LazyLock::new(|| {
    HashMap::from([
        // USD - United States Dollar
        ("usd", Currency::USD),
        ("united states", Currency::USD),
        ("usa", Currency::USD),
        ("us", Currency::USD),
        ("america", Currency::USD),
        ("american", Currency::USD),
        // EUR - Euro
        ("eur", Currency::EUR),
        ("euro", Currency::EUR),
        ("eurozone", Currency::EUR),
        ("european", Currency::EUR),
        // GBP - British Pound
        ("gbp", Currency::GBP),
        ("united kingdom", Currency::GBP),
        ("uk", Currency::GBP),
        ("britain", Currency::GBP),
        ("british", Currency::GBP),
        ("england", Currency::GBP),
        ("english", Currency::GBP),
        // JPY - Japanese Yen
        ("jpy", Currency::JPY),
        ("japan", Currency::JPY),
        ("japanese", Currency::JPY),
        // AUD - Australian Dollar
        ("aud", Currency::AUD),
        ("australia", Currency::AUD),
        ("australian", Currency::AUD),
        // CAD - Canadian Dollar
        ("cad", Currency::CAD),
        ("canada", Currency::CAD),
        ("canadian", Currency::CAD),
        // CHF - Swiss Franc
        ("chf", Currency::CHF),
        ("switzerland", Currency::CHF),
        ("swiss", Currency::CHF),
        // NZD - New Zealand Dollar
        ("nzd", Currency::NZD),
        ("new zealand", Currency::NZD),
        ("kiwi", Currency::NZD),
        // CNY/CNH - Chinese Yuan
        ("cny", Currency::CNY),
        ("cnh", Currency::CNY),
        ("china", Currency::CNY),
        ("chinese", Currency::CNY),
    ])
});

/// Resolve an alias to a Currency enum
fn resolve_currency_enum(input: &str) -> Option<Currency> {
    let normalized = input.trim().to_lowercase();
    ALIAS_TO_CURRENCY.get(normalized.as_str()).copied()
}

/// Resolve a currency input to its ISO 4217 code string.
///
/// Accepts:
/// - Currency codes: "USD", "usd" → "USD"
/// - Country names: "Canada", "CANADA" → "CAD"
/// - Demonyms: "Canadian", "american" → respective codes
///
/// Returns the input uppercased if no mapping found (for backwards compatibility).
pub fn resolve_currency(input: &str) -> String {
    resolve_currency_enum(input)
        .map(|c| c.code().to_string())
        .unwrap_or_else(|| input.trim().to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_codes_pass_through() {
        assert_eq!(resolve_currency("USD"), "USD");
        assert_eq!(resolve_currency("usd"), "USD");
        assert_eq!(resolve_currency("cad"), "CAD");
        assert_eq!(resolve_currency("EUR"), "EUR");
    }

    #[test]
    fn test_country_names_resolve() {
        assert_eq!(resolve_currency("Canada"), "CAD");
        assert_eq!(resolve_currency("CANADA"), "CAD");
        assert_eq!(resolve_currency("canada"), "CAD");
        assert_eq!(resolve_currency("United States"), "USD");
        assert_eq!(resolve_currency("Japan"), "JPY");
        assert_eq!(resolve_currency("Australia"), "AUD");
    }

    #[test]
    fn test_demonyms_resolve() {
        assert_eq!(resolve_currency("Canadian"), "CAD");
        assert_eq!(resolve_currency("American"), "USD");
        assert_eq!(resolve_currency("Japanese"), "JPY");
    }

    #[test]
    fn test_unknown_input_uppercased() {
        // Unknown inputs should just be uppercased (backwards compatible)
        assert_eq!(resolve_currency("xyz"), "XYZ");
        assert_eq!(resolve_currency("Unknown"), "UNKNOWN");
    }

    #[test]
    fn test_whitespace_handling() {
        assert_eq!(resolve_currency("  USD  "), "USD");
        assert_eq!(resolve_currency("  canada  "), "CAD");
    }
}
