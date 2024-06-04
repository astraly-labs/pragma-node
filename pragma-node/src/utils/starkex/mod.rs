use bigdecimal::BigDecimal;

use starknet::{
    core::{
        crypto::{pedersen_hash, EcdsaSignError, Signature},
        types::FieldElement,
    },
    signers::SigningKey,
};

#[allow(dead_code)]
pub fn get_price_message(
    _oracle_name: &str,
    _pair_id: &str,
    _timestamp: u64,
    _price: BigDecimal,
) -> FieldElement {
    // 1. Build number A from oracle_name & pair_id
    let a = FieldElement::from_hex_be("425443555344000000000000000000004d616b6572").unwrap();
    // 2. Build number B from price & timestamp
    let b = FieldElement::from_hex_be("27015cfcb02308200005e0be100").unwrap();
    pedersen_hash(&a, &b)
}

/// Sign the hashed_data using the private_key.
/// See get_price_message for hashed_data context.
///
/// TODO: Assumes we already have a private key registered.g
///
/// E.g:
/// key = 178047D3869489C055D7EA54C014FFB834A069C9595186ABE04EA4D1223A03F
/// data = 3e4113feb6c403cb0c954e5c09d239bf88fedb075220270f44173ac3cd41858
/// gives
/// r: 0x6a7a118a6fa508c4f0eb77ea0efbc8d48a64d4a570d93f5c61cd886877cb920
/// s: 0x6de9006a7bbf610d583d514951c98d15b1a0f6c78846986491d2c8ca049fd55
#[allow(dead_code)]
pub fn sign(
    private_key: FieldElement,
    hashed_data: FieldElement,
) -> Result<Signature, EcdsaSignError> {
    let signing_key = SigningKey::from_secret_scalar(private_key);
    signing_key.sign(&hashed_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::{BigDecimal, FromPrimitive};

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
        let hashed_data = FieldElement::from_hex_be(
            "3e4113feb6c403cb0c954e5c09d239bf88fedb075220270f44173ac3cd41858",
        )
        .unwrap();

        // 2. Action
        let signature = sign(private_key, hashed_data).unwrap();

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
