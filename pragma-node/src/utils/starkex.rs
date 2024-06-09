use bigdecimal::{BigDecimal, ToPrimitive};

use starknet::core::{
    crypto::pedersen_hash, types::FieldElement, utils::cairo_short_string_to_felt,
};

#[derive(Debug)]
pub enum HashError {
    ConversionError,
}

/// Converts oracle name and pair id to an external asset id.
pub fn get_external_asset_id(oracle_name: &str, pair_id: &str) -> Result<String, HashError> {
    let oracle_name =
        cairo_short_string_to_felt(oracle_name).map_err(|_| HashError::ConversionError)?;
    let oracle_as_hex = format!("{:x}", oracle_name);
    let pair_id = cairo_short_string_to_felt(pair_id).map_err(|_| HashError::ConversionError)?;
    let pair_id: u128 = pair_id.try_into().map_err(|_| HashError::ConversionError)?;
    let pair_as_hex = format!("{:0<width$x}", pair_id, width = 32);
    Ok(format!("{}{}", pair_as_hex, oracle_as_hex))
}

/// Builds the second number for the hash computation based on timestamp and price.
fn build_second_number(timestamp: u128, price: &BigDecimal) -> Result<FieldElement, HashError> {
    let price = price.to_u128().ok_or(HashError::ConversionError)?;
    let price_as_hex = format!("{:x}", price);
    let timestamp_as_hex = format!("{:x}", timestamp);
    let v = format!("{}{}", price_as_hex, timestamp_as_hex);
    FieldElement::from_hex_be(&v).map_err(|_| HashError::ConversionError)
}

/// Computes a signature-ready message based on oracle, asset, timestamp
/// and price.
/// The signature is the pedersen hash of two FieldElements:
///
/// first number (external_asset_id):
///  ---------------------------------------------------------------------------------
///  | asset_name (rest of the number)  - 211 bits       |   oracle_name (40 bits)   |
///  ---------------------------------------------------------------------------------
///
/// second number:
///  ---------------------------------------------------------------------------------
///  | 0 (92 bits)         | price (120 bits)              |   timestamp (32 bits)   |
///  ---------------------------------------------------------------------------------
///
/// See:
/// https://docs.starkware.co/starkex/perpetual/becoming-an-oracle-provider-for-starkex.html#signing_prices
pub fn get_entry_hash(
    oracle_name: &str,
    pair_id: &str,
    timestamp: u64,
    price: &BigDecimal,
) -> Result<FieldElement, HashError> {
    let external_asset_id = get_external_asset_id(oracle_name, pair_id)?;
    let first_number =
        FieldElement::from_hex_be(&external_asset_id).map_err(|_| HashError::ConversionError)?;
    let second_number = build_second_number(timestamp as u128, price)?;
    Ok(pedersen_hash(&first_number, &second_number))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use bigdecimal::BigDecimal;

    // Example from:
    // https://docs.starkware.co/starkex/perpetual/becoming-an-oracle-provider-for-starkex.html#signing_prices
    #[test]
    fn test_get_entry_hash_with_example() {
        // 1. Setup
        let oracle_name = "Maker";
        let asset = "BTCUSD";
        let price = BigDecimal::from_str("11512340000000000000000").unwrap();
        let timestamp = 1577836800_u64;

        // 2. Action
        let hashed_data =
            get_entry_hash(oracle_name, asset, timestamp, &price).expect("Could not build hash");

        // 3. Check
        let expected_data = FieldElement::from_hex_be(
            "3e4113feb6c403cb0c954e5c09d239bf88fedb075220270f44173ac3cd41858",
        )
        .unwrap();
        assert_eq!(hashed_data, expected_data);
    }

    // TODO(akhercha): do way more tests
}
