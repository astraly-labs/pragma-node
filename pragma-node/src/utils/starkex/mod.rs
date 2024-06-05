use bigdecimal::{BigDecimal, ToPrimitive};

use starknet::{
    core::{
        crypto::{pedersen_hash, EcdsaSignError, Signature},
        types::FieldElement,
        utils::cairo_short_string_to_felt,
    },
    signers::SigningKey,
};

fn build_first_number(oracle_name: &str, pair_id: &str) -> FieldElement {
    let oracle_name = cairo_short_string_to_felt(oracle_name).unwrap();
    let oracle_as_hex = format!("{:x}", oracle_name);
    let pair_id = cairo_short_string_to_felt(pair_id).unwrap();
    let pair_id: u128 = pair_id.try_into().unwrap();
    // 32 bytes padding corresponding to 128 bits
    let pair_as_hex = format!("{:0<width$x}", pair_id, width = 32);
    let v = format!("{}{}", pair_as_hex, oracle_as_hex);
    FieldElement::from_hex_be(&v).unwrap()
}

fn build_second_number(timestamp: u64, price: BigDecimal) -> FieldElement {
    // TODO(akhercha): round?
    let price = price.round(2);
    // TODO(akhercha): 18 all the time ? Or can be different depending on pairs?
    let price = price * BigDecimal::from(10_u128.pow(18));
    let price = price.to_u128().unwrap();
    let price_as_hex = format!("{:x}", price);
    let timestamp: u128 = timestamp.into();
    let timestamp_as_hex = format!("{:x}", timestamp);
    let v = format!("{}{}", price_as_hex, timestamp_as_hex);
    FieldElement::from_hex_be(&v).unwrap()
}

/// Computes a signature-ready message based on oracle, asset, timestamp
/// and price.
/// The signature is the pedersen hash of two FieldElements:
///
/// first number:
///  ---------------------------------------------------------------------------------
///  | asset_name (rest of the number)  - 211 bits       |   oracle_name (40 bits)   |
///  ---------------------------------------------------------------------------------
///
/// second number:
///  ---------------------------------------------------------------------------------
///  | 0 (92 bits)         | price (120 bits)              |   timestamp (32 bits)   |
///  ---------------------------------------------------------------------------------
///
#[allow(dead_code)]
pub fn get_price_message(
    // TODO(akhercha): oracle name should be a constant "Pragma"
    oracle_name: &str,
    pair_id: &str,
    timestamp: u64,
    price: BigDecimal,
) -> FieldElement {
    let first_number = build_first_number(oracle_name, pair_id);
    let second_number = build_second_number(timestamp, price);
    pedersen_hash(&first_number, &second_number)
}

/// Sign the hashed_data using the private_key.
#[allow(dead_code)]
pub fn sign(
    signing_key: SigningKey,
    hashed_data: FieldElement,
) -> Result<Signature, EcdsaSignError> {
    signing_key.sign(&hashed_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::{BigDecimal, FromPrimitive};

    // TODO(akhercha): generate more tests with the CLI
    #[test]
    fn test_get_price_message_with_example() {
        // 1. Setup
        let oracle_name = "Maker";
        let asset = "BTCUSD";
        let price = BigDecimal::from_f64(11512.34).unwrap();
        let timestamp = 1577836800_u64;

        // 2. Action
        let hashed_data = get_price_message(oracle_name, asset, timestamp, price);

        // 3. Check
        let expected_data = FieldElement::from_hex_be(
            "3e4113feb6c403cb0c954e5c09d239bf88fedb075220270f44173ac3cd41858",
        )
        .unwrap();
        assert_eq!(hashed_data, expected_data);
    }

    #[test]
    fn test_sign_with_example() {
        // 1. Setup
        let private_key = FieldElement::from_hex_be(
            "178047D3869489C055D7EA54C014FFB834A069C9595186ABE04EA4D1223A03F",
        )
        .unwrap();
        let signing_key = SigningKey::from_secret_scalar(private_key);
        let hashed_data = FieldElement::from_hex_be(
            "3e4113feb6c403cb0c954e5c09d239bf88fedb075220270f44173ac3cd41858",
        )
        .unwrap();

        // 2. Action
        let signature = sign(signing_key, hashed_data).unwrap();

        // 3. Check
        let expected_r = FieldElement::from_hex_be(
            "6a7a118a6fa508c4f0eb77ea0efbc8d48a64d4a570d93f5c61cd886877cb920",
        )
        .unwrap();
        let expected_s = FieldElement::from_hex_be(
            "6de9006a7bbf610d583d514951c98d15b1a0f6c78846986491d2c8ca049fd55",
        )
        .unwrap();

        assert_eq!(signature.r, expected_r);
        assert_eq!(signature.s, expected_s);
    }
}
