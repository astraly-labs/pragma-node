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
