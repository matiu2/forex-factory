//! Currency code resolution from country names and abbreviations.
//!
//! Supports resolving inputs like "Canada", "Canadian", "CAD" all to "CAD".

use std::collections::HashMap;
use std::sync::LazyLock;

/// Static mapping of country names/variations to ISO 4217 currency codes.
/// Keys are lowercase for case-insensitive lookup.
static COUNTRY_TO_CURRENCY: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        // USD - United States Dollar
        ("usd", "USD"),
        ("united states", "USD"),
        ("usa", "USD"),
        ("us", "USD"),
        ("america", "USD"),
        ("american", "USD"),
        // EUR - Euro
        ("eur", "EUR"),
        ("euro", "EUR"),
        ("eurozone", "EUR"),
        ("european", "EUR"),
        // GBP - British Pound
        ("gbp", "GBP"),
        ("united kingdom", "GBP"),
        ("uk", "GBP"),
        ("britain", "GBP"),
        ("british", "GBP"),
        ("england", "GBP"),
        ("english", "GBP"),
        // JPY - Japanese Yen
        ("jpy", "JPY"),
        ("japan", "JPY"),
        ("japanese", "JPY"),
        // AUD - Australian Dollar
        ("aud", "AUD"),
        ("australia", "AUD"),
        ("australian", "AUD"),
        // CAD - Canadian Dollar
        ("cad", "CAD"),
        ("canada", "CAD"),
        ("canadian", "CAD"),
        // CHF - Swiss Franc
        ("chf", "CHF"),
        ("switzerland", "CHF"),
        ("swiss", "CHF"),
        // NZD - New Zealand Dollar
        ("nzd", "NZD"),
        ("new zealand", "NZD"),
        ("kiwi", "NZD"),
        // CNY/CNH - Chinese Yuan
        ("cny", "CNY"),
        ("cnh", "CNY"),
        ("china", "CNY"),
        ("chinese", "CNY"),
    ])
});

/// Resolve a currency input to its ISO 4217 code.
///
/// Accepts:
/// - Currency codes: "USD", "usd" → "USD"
/// - Country names: "Canada", "CANADA" → "CAD"
/// - Demonyms: "Canadian", "american" → respective codes
///
/// Returns the input uppercased if no mapping found (for backwards compatibility).
pub fn resolve_currency(input: &str) -> String {
    let normalized = input.trim().to_lowercase();

    COUNTRY_TO_CURRENCY
        .get(normalized.as_str())
        .map(|&code| code.to_string())
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
