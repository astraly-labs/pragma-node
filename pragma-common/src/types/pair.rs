use std::str::FromStr;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A pair of assets, e.g. BTC/USD
///
/// This is a simple struct that holds the base and quote assets.
/// It is used to represent a pair of assets in the system.
/// Base and quote are always in UPPERCASE.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct Pair {
    pub base: String,
    pub quote: String,
}

impl Pair {
    /// Creates a routed pair from two pairs that share a common quote currency.
    ///
    /// e.g. "BTC/USD" and "ETH/USD" -> "BTC/ETH"
    pub fn create_routed_pair(base_pair: &Pair, quote_pair: &Pair) -> Self {
        Self {
            base: base_pair.base.clone(),
            quote: quote_pair.base.clone(),
        }
    }

    /// Creates a new pair from base and quote currencies.
    pub fn from_currencies(base: &str, quote: &str) -> Self {
        Self {
            base: base.to_uppercase(),
            quote: quote.to_uppercase(),
        }
    }

    /// Get the base and quote as a tuple
    pub fn as_tuple(&self) -> (String, String) {
        (self.base.clone(), self.quote.clone())
    }

    /// Format pair with a custom separator
    pub fn format_with_separator(&self, separator: &str) -> String {
        format!("{}{}{}", self.base, separator, self.quote)
    }

    /// Get the pair ID in standard format without consuming self
    pub fn to_pair_id(&self) -> String {
        self.format_with_separator("/")
    }
}

impl std::fmt::Display for Pair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.base, self.quote)
    }
}

impl From<Pair> for String {
    fn from(pair: Pair) -> Self {
        format!("{0}/{1}", pair.base, pair.quote)
    }
}

impl From<&str> for Pair {
    fn from(pair_id: &str) -> Self {
        let normalized = pair_id.replace(['-', '_'], "/");
        let parts: Vec<&str> = normalized.split('/').collect();
        Self {
            base: parts[0].trim().to_uppercase(),
            quote: parts[1].trim().to_uppercase(),
        }
    }
}

impl From<String> for Pair {
    fn from(pair_id: String) -> Self {
        Self::from(pair_id.as_str())
    }
}

impl FromStr for Pair {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

impl From<(String, String)> for Pair {
    fn from(pair: (String, String)) -> Self {
        Self {
            base: pair.0.to_uppercase(),
            quote: pair.1.to_uppercase(),
        }
    }
}

#[macro_export]
macro_rules! pair {
    ($pair_str:expr) => {{
        #[allow(dead_code)]
        const fn validate_pair(s: &str) -> bool {
            let mut count = 0;
            let chars = s.as_bytes();
            let mut i = 0;
            while i < chars.len() {
                if chars[i] == b'/' || chars[i] == b'-' || chars[i] == b'_' {
                    count += 1;
                }
                i += 1;
            }
            count == 1
        }
        const _: () = {
            assert!(
                validate_pair($pair_str),
                "Invalid pair format. Expected format: BASE/QUOTE, BASE-QUOTE, or BASE_QUOTE"
            );
        };
        let normalized = $pair_str.replace('-', "/").replace('_', "/");
        let parts: Vec<&str> = normalized.split('/').collect();
        Pair {
            base: parts[0].trim().to_uppercase(),
            quote: parts[1].trim().to_uppercase(),
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pair_macro() {
        let pair1 = pair!("BTC/USD");
        assert_eq!(pair1.base, "BTC");
        assert_eq!(pair1.quote, "USD");

        let pair2 = pair!("btc-usd");
        assert_eq!(pair2.base, "BTC");
        assert_eq!(pair2.quote, "USD");

        let pair3 = pair!("eth_usdt");
        assert_eq!(pair3.base, "ETH");
        assert_eq!(pair3.quote, "USDT");

        let pair4 = pair!("SOL/usdc");
        assert_eq!(pair4.base, "SOL");
        assert_eq!(pair4.quote, "USDC");

        let pair5 = pair!("bTc/uSDt");
        assert_eq!(pair5.base, "BTC");
        assert_eq!(pair5.quote, "USDT");
    }

    #[test]
    fn test_pair_conversions() {
        // Test from_currencies
        let pair = Pair::from_currencies("btc", "usd");
        assert_eq!(pair.base, "BTC");
        assert_eq!(pair.quote, "USD");

        // Test create_routed_pair
        let btc_usd = Pair::from_currencies("btc", "usd");
        let eth_usd = Pair::from_currencies("eth", "usd");
        let btc_eth = Pair::create_routed_pair(&btc_usd, &eth_usd);
        assert_eq!(btc_eth.base, "BTC");
        assert_eq!(btc_eth.quote, "ETH");

        // Test From<&str>
        let pair_from_str = Pair::from("btc-usd");
        assert_eq!(pair_from_str.base, "BTC");
        assert_eq!(pair_from_str.quote, "USD");

        let pair_from_str = Pair::from("ETH_USDT");
        assert_eq!(pair_from_str.base, "ETH");
        assert_eq!(pair_from_str.quote, "USDT");

        let pair_from_str = Pair::from("BTC/USD");
        assert_eq!(pair_from_str.base, "BTC");
        assert_eq!(pair_from_str.quote, "USD");

        // Test From<(String, String)>
        let pair_from_tuple = Pair::from((String::from("btc"), String::from("usdt")));
        assert_eq!(pair_from_tuple.base, "BTC");
        assert_eq!(pair_from_tuple.quote, "USDT");

        // Using into()
        let pair_from_tuple: Pair = (String::from("eth"), String::from("usdc")).into();
        assert_eq!(pair_from_tuple.base, "ETH");
        assert_eq!(pair_from_tuple.quote, "USDC");

        // Test as_tuple()
        let pair = Pair::from_currencies("btc", "usd");
        let (base, quote) = pair.as_tuple();
        assert_eq!(base, "BTC");
        assert_eq!(quote, "USD");

        // Test format_with_separator
        let pair = Pair::from_currencies("btc", "usd");
        assert_eq!(pair.format_with_separator("/"), "BTC/USD");
        assert_eq!(pair.format_with_separator("-"), "BTC-USD");
        assert_eq!(pair.format_with_separator("_"), "BTC_USD");
        assert_eq!(pair.to_string(), "BTC/USD");
    }

    // This test is commented out because it would fail at compile time
    // #[test]
    // fn test_invalid_pair() {
    //     let _pair = generate_pair!("BTC/USD/EUR"); // This will fail at compile time
    // }
}
