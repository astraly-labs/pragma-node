use bigdecimal::{BigDecimal, ToPrimitive};

use starknet::core::{
    crypto::pedersen_hash, types::FieldElement, utils::cairo_short_string_to_felt,
};

#[derive(Debug)]
pub enum HashError {
    ConversionError,
}

/// Converts a pair id to its hexadecimal id.
pub fn get_encoded_pair_id(pair_id: &str) -> Result<String, HashError> {
    let pair_id = pair_id.replace('/', ""); // Remove the "/" from the pair_id if it exists
    let pair_id = cairo_short_string_to_felt(&pair_id).map_err(|_| HashError::ConversionError)?;
    Ok(format!("0x{:x}", pair_id))
}

/// Converts oracle name and pair id to an external asset id.
fn build_first_number(oracle_name: &str, pair_id: &str) -> Result<FieldElement, HashError> {
    let oracle_name =
        cairo_short_string_to_felt(oracle_name).map_err(|_| HashError::ConversionError)?;
    let oracle_as_hex = format!("{:x}", oracle_name);
    let pair_id = cairo_short_string_to_felt(pair_id).map_err(|_| HashError::ConversionError)?;
    let pair_id: u128 = pair_id.try_into().map_err(|_| HashError::ConversionError)?;
    let pair_as_hex = format!("{:0<width$x}", pair_id, width = 32);
    let v = format!("0x{}{}", pair_as_hex, oracle_as_hex);
    FieldElement::from_hex_be(&v).map_err(|_| HashError::ConversionError)
}

/// Builds the second number for the hash computation based on timestamp and price.
fn build_second_number(timestamp: u128, price: &BigDecimal) -> Result<FieldElement, HashError> {
    let price = price.to_u128().ok_or(HashError::ConversionError)?;
    let price_as_hex = format!("{:x}", price);
    let timestamp_as_hex = format!("{:x}", timestamp);
    let v = format!("0x{}{}", price_as_hex, timestamp_as_hex);
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
    let pair_id = pair_id.replace('/', ""); // Remove the "/" from the pair_id if it exists
    let first_number = build_first_number(oracle_name, &pair_id)?;
    let second_number = build_second_number(timestamp as u128, price)?;
    Ok(pedersen_hash(&first_number, &second_number))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use bigdecimal::BigDecimal;

    // (pair_id, expected_encoded_pair_id)
    type GetEncodedPairIdTestCase<'a> = (&'a str, &'a str);

    #[test]
    fn test_get_encoded_pair_id() {
        let tests_cases: Vec<GetEncodedPairIdTestCase> = vec![
            ("BTCUSD", "0x425443555344"),
            ("BTC/USD", "0x425443555344"),
            ("ETHUSD", "0x455448555344"),
            ("DOGEUSD", "0x444f4745555344"),
            ("SOLUSD", "0x534f4c555344"),
            ("SOLUSDT", "0x534f4c55534454"),
        ];

        for (pair_id, expected_encoded_pair_id) in tests_cases {
            let encoded_pair_id = get_encoded_pair_id(pair_id).expect("Could not encode pair id");
            assert_eq!(
                encoded_pair_id, expected_encoded_pair_id,
                "Encoded pair id does not match for pair_id: {}",
                pair_id
            );
        }
    }

    // ((oracle_name, pair_id, price, timestamp), expected_hash)
    type GetEntryHashTestCase<'a> = ((&'a str, &'a str, &'a str, u64), &'a str);

    #[test]
    fn test_get_entry_hash() {
        let tests_cases: Vec<GetEntryHashTestCase> = vec![
            (
                ("Maker", "BTCUSD", "11512340000000000000000", 1577836800),
                "3e4113feb6c403cb0c954e5c09d239bf88fedb075220270f44173ac3cd41858",
            ),
            (
                ("Maker", "BTC/USD", "11512340000000000000000", 1577836800),
                "3e4113feb6c403cb0c954e5c09d239bf88fedb075220270f44173ac3cd41858",
            ),
            (
                ("PRGM", "SOLUSD", "19511280076", 1577216800),
                "3d683d36601ab3fd05dfbfecea8971a798f3c2e418fa54594c363e6e6816979",
            ),
            (
                ("PRGM", "ETHUSD", "369511280076", 1577816800),
                "6641dffd4e3499051ca0cd57e5c12b203bcf184576ce72e18d832de941e9656",
            ),
            (
                ("TEST", "DOGEUSD", "51128006", 1517816800),
                "18320fa96c61b1d8f98e1c85ae0a5a1159a46580ad32415122661c470d8d99f",
            ),
            (
                ("TEST", "DOGE/USD", "51128006", 1517816800),
                "18320fa96c61b1d8f98e1c85ae0a5a1159a46580ad32415122661c470d8d99f",
            ),
        ];

        for ((oracle_name, pair_id, price, timestamp), expected_hash) in tests_cases {
            let price = BigDecimal::from_str(price).unwrap();
            let hashed_data = get_entry_hash(oracle_name, pair_id, timestamp, &price)
                .expect("Could not build hash");
            let expected_data = FieldElement::from_hex_be(expected_hash).unwrap();
            assert_eq!(
                hashed_data, expected_data,
                "Hashes do not match for oracle_name: {}, pair_id: {}, price: {}, timestamp: {}",
                oracle_name, pair_id, price, timestamp
            );
        }
    }
}
