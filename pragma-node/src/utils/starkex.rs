use bigdecimal::{BigDecimal, ToPrimitive};

use starknet::core::{
    crypto::pedersen_hash, types::FieldElement, utils::cairo_short_string_to_felt,
};

/// TODO(akhercha): Write this.
pub fn get_external_asset_id(oracle_name: &str, pair_id: &str) -> String {
    // TODO(akhercha): unsafe unwrap
    let oracle_name = cairo_short_string_to_felt(oracle_name).unwrap();
    let oracle_as_hex = format!("{:x}", oracle_name);
    // TODO(akhercha): unsafe unwrap
    let pair_id = cairo_short_string_to_felt(pair_id).unwrap();
    let pair_id: u128 = pair_id.try_into().unwrap();
    // 32 bytes padding corresponding to 128 bits
    let pair_as_hex = format!("{:0<width$x}", pair_id, width = 32);
    format!("{}{}", pair_as_hex, oracle_as_hex)
}

/// TODO(akhercha): Write this.
fn build_second_number(timestamp: u64, price: &BigDecimal) -> FieldElement {
    // TODO(akhercha): round?
    let price = price.round(2);
    // TODO(akhercha): 18 all the time ? Or can be different depending on pairs?
    let price = price * BigDecimal::from(10_u128.pow(18));
    // TODO(akhercha): unsafe unwrap
    let price = price.to_u128().unwrap();
    let price_as_hex = format!("{:x}", price);
    let timestamp: u128 = timestamp.into();
    let timestamp_as_hex = format!("{:x}", timestamp);
    let v = format!("{}{}", price_as_hex, timestamp_as_hex);
    // TODO(akhercha): unsafe unwrap
    FieldElement::from_hex_be(&v).unwrap()
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
pub fn get_entry_hash(
    // TODO(akhercha): oracle name should be a constant "Pragma"
    oracle_name: &str,
    pair_id: &str,
    timestamp: u64,
    price: &BigDecimal,
) -> FieldElement {
    // TODO(akhercha): unsafe unwrap
    let external_asset_id = get_external_asset_id(oracle_name, pair_id);
    // TODO(akhercha): unsafe unwrap
    let first_number = FieldElement::from_hex_be(&external_asset_id).unwrap();
    let second_number = build_second_number(timestamp, price);
    pedersen_hash(&first_number, &second_number)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::{BigDecimal, FromPrimitive};

    // TODO(akhercha): do way more tests
    #[test]
    fn test_get_entry_hash_with_example() {
        // 1. Setup
        let oracle_name = "Maker";
        let asset = "BTCUSD";
        let price = BigDecimal::from_f64(11512.34).unwrap();
        let timestamp = 1577836800_u64;

        // 2. Action
        let hashed_data = get_entry_hash(oracle_name, asset, timestamp, &price);

        // 3. Check
        let expected_data = FieldElement::from_hex_be(
            "3e4113feb6c403cb0c954e5c09d239bf88fedb075220270f44173ac3cd41858",
        )
        .unwrap();
        assert_eq!(hashed_data, expected_data);
    }
}
