use std::str::FromStr;

use bigdecimal::BigDecimal;

use pragma_entities::InfraError;
use serde::{Deserialize, Deserializer};
use starknet_crypto::Felt;

pub fn convert_via_quote(
    a_price: BigDecimal,
    b_price: BigDecimal,
    output_decimals: u32,
) -> Result<BigDecimal, InfraError> {
    if b_price == BigDecimal::from(0) {
        return Err(InfraError::InternalServerError);
    }

    let power = BigDecimal::from(10_i64.pow(output_decimals));

    Ok(a_price * power / b_price)
}

pub fn normalize_to_decimals(
    value: BigDecimal,
    original_decimals: u32,
    target_decimals: u32,
) -> BigDecimal {
    if target_decimals >= original_decimals {
        let power = BigDecimal::from(10_i64.pow(target_decimals - original_decimals));
        value * power
    } else {
        let power = BigDecimal::from(10_i64.pow(original_decimals - target_decimals));
        value / power
    }
}

pub fn format_bigdecimal_price(price: BigDecimal, decimals: u32) -> String {
    let price_decimal = BigDecimal::from_str(&price.to_string()).unwrap();
    let scale_factor = BigDecimal::from(10u64.pow(decimals));
    let adjusted_price = &price_decimal / &scale_factor;
    let mut formatted_price = adjusted_price.to_string();
    if formatted_price.contains('.') {
        while formatted_price.ends_with('0') {
            formatted_price.pop();
        }
        if formatted_price.ends_with('.') {
            formatted_price.pop();
        }
    }
    formatted_price
}

pub fn felt_from_decimal<'de, D>(deserializer: D) -> Result<Vec<Felt>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Vec<String> = Vec::deserialize(deserializer)?;
    Ok(s.iter().map(|s| Felt::from_dec_str(s).unwrap()).collect())
}

/// Converts two currencies pairs to a new routed pair id.
///
/// e.g "btc/usd" and "eth/usd" to "btc/eth"
pub fn currency_pairs_to_routed_pair_id(base_pair: &str, quote_pair: &str) -> String {
    let (base, _) = pair_id_to_currency_pair(base_pair);
    let (quote, _) = pair_id_to_currency_pair(quote_pair);
    format!("{}/{}", base.to_uppercase(), quote.to_uppercase())
}

/// Converts a currency pair to a pair id.
///
/// e.g "btc" and "usd" to "BTC/USD"
pub fn currency_pair_to_pair_id(base: &str, quote: &str) -> String {
    format!("{}/{}", base.to_uppercase(), quote.to_uppercase())
}

/// Converts a pair_id to a currency pair.
///
/// e.g "BTC/USD" to ("BTC", "USD")
pub fn pair_id_to_currency_pair(pair_id: &str) -> (String, String) {
    let parts: Vec<&str> = pair_id.split('/').collect();
    (parts[0].to_string(), parts[1].to_string())
}
