use bigdecimal::{BigDecimal, ToPrimitive};
use starknet::{
    core::{
        crypto::{pedersen_hash, EcdsaSignError},
        types::FieldElement,
        utils::cairo_short_string_to_felt,
    },
    signers::SigningKey,
};

use pragma_common::types::ConversionError;

use crate::handlers::entries::constants::PRAGMA_ORACLE_NAME_FOR_STARKEX;

use super::sign_data;

pub enum SigningError {
    ConversionError,
    SigningError(EcdsaSignError),
}

pub fn sign_median_price(
    signer: &SigningKey,
    asset_id: &str,
    timestamp: u64,
    median_price: BigDecimal,
) -> Result<String, SigningError> {
    let hash_to_sign = get_entry_hash(
        PRAGMA_ORACLE_NAME_FOR_STARKEX,
        asset_id,
        timestamp,
        &median_price,
    )
    .map_err(|_| SigningError::ConversionError)?;
    let signature = sign_data(signer, hash_to_sign).map_err(SigningError::SigningError)?;
    Ok(format!("0x{:}", signature))
}

/// Converts a pair id to its hexadecimal id.
pub fn get_global_asset_it(pair_id: &str) -> Result<String, ConversionError> {
    let pair_id = pair_id.replace('/', ""); // Remove the "/" from the pair_id if it exists
    let pair_id =
        cairo_short_string_to_felt(&pair_id).map_err(|_| ConversionError::FeltConversion)?;
    Ok(format!("0x{:x}", pair_id))
}

/// Computes a signature-ready message based on oracle, asset, timestamp
/// and price.
/// The signature is the pedersen hash of two FieldElements:
///
/// first number (oracle_asset_id):
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
fn get_entry_hash(
    oracle_name: &str,
    pair_id: &str,
    timestamp: u64,
    price: &BigDecimal,
) -> Result<FieldElement, ConversionError> {
    let first_number = build_external_asset_id(oracle_name, pair_id)?;
    let second_number = build_second_number(timestamp as u128, price)?;
    Ok(pedersen_hash(&first_number, &second_number))
}

pub fn get_oracle_asset_id(oracle_name: &str, pair_id: &str) -> Result<String, ConversionError> {
    let pair_id = pair_id.replace('/', ""); // Remove the "/" from the pair_id if it exists
    let oracle_name =
        cairo_short_string_to_felt(oracle_name).map_err(|_| ConversionError::FeltConversion)?;
    let oracle_as_hex = format!("{:x}", oracle_name);
    let pair_id =
        cairo_short_string_to_felt(&pair_id).map_err(|_| ConversionError::FeltConversion)?;
    let pair_id: u128 = pair_id
        .try_into()
        .map_err(|_| ConversionError::U128Conversion)?;
    let pair_as_hex = format!("{:0<width$x}", pair_id, width = 32);
    Ok(format!("0x{}{}", pair_as_hex, oracle_as_hex))
}

/// Builds the first number for the hash computation based on oracle name and pair id.
fn build_external_asset_id(
    oracle_name: &str,
    pair_id: &str,
) -> Result<FieldElement, ConversionError> {
    let external_asset_id = get_oracle_asset_id(oracle_name, pair_id)?;
    FieldElement::from_hex_be(&external_asset_id).map_err(|_| ConversionError::FeltConversion)
}

/// Builds the second number for the hash computation based on timestamp and price.
fn build_second_number(
    timestamp: u128,
    price: &BigDecimal,
) -> Result<FieldElement, ConversionError> {
    let price = price.to_u128().ok_or(ConversionError::U128Conversion)?;
    let price_as_hex = format!("{:x}", price);
    let timestamp_as_hex = format!("{:x}", timestamp);
    let v = format!("0x{}{}", price_as_hex, timestamp_as_hex);
    FieldElement::from_hex_be(&v).map_err(|_| ConversionError::FeltConversion)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use std::str::FromStr;

    use super::*;
    use bigdecimal::BigDecimal;

    #[rstest]
    #[case("BTCUSD", "0x425443555344")]
    #[case("BTC/USD", "0x425443555344")]
    #[case("ETHUSD", "0x455448555344")]
    #[case("DOGEUSD", "0x444f4745555344")]
    #[case("SOLUSD", "0x534f4c555344")]
    #[case("SOLUSDT", "0x534f4c55534454")]
    fn test_get_encoded_pair_id(#[case] pair_id: &str, #[case] expected_encoded_pair_id: &str) {
        let encoded_pair_id = get_global_asset_it(pair_id).expect("Could not encode pair id");
        assert_eq!(
            encoded_pair_id, expected_encoded_pair_id,
            "Encoded pair id does not match for pair_id: {}",
            pair_id
        );
    }

    #[rstest]
    #[case(
        "Maker",
        "BTCUSD",
        "11512340000000000000000",
        1577836800,
        "3e4113feb6c403cb0c954e5c09d239bf88fedb075220270f44173ac3cd41858"
    )]
    #[case(
        "Maker",
        "BTC/USD",
        "11512340000000000000000",
        1577836800,
        "3e4113feb6c403cb0c954e5c09d239bf88fedb075220270f44173ac3cd41858"
    )]
    #[case(
        "PRGM",
        "SOLUSD",
        "19511280076",
        1577216800,
        "3d683d36601ab3fd05dfbfecea8971a798f3c2e418fa54594c363e6e6816979"
    )]
    #[case(
        "PRGM",
        "ETHUSD",
        "369511280076",
        1577816800,
        "6641dffd4e3499051ca0cd57e5c12b203bcf184576ce72e18d832de941e9656"
    )]
    #[case(
        "TEST",
        "DOGEUSD",
        "51128006",
        1517816800,
        "18320fa96c61b1d8f98e1c85ae0a5a1159a46580ad32415122661c470d8d99f"
    )]
    #[case(
        "TEST",
        "DOGE/USD",
        "51128006",
        1517816800,
        "18320fa96c61b1d8f98e1c85ae0a5a1159a46580ad32415122661c470d8d99f"
    )]
    fn test_get_entry_hash(
        #[case] oracle_name: &str,
        #[case] pair_id: &str,
        #[case] price: &str,
        #[case] timestamp: u64,
        #[case] expected_hash: &str,
    ) {
        let price = BigDecimal::from_str(price).unwrap();
        let hashed_data =
            get_entry_hash(oracle_name, pair_id, timestamp, &price).expect("Could not build hash");
        let expected_data = FieldElement::from_hex_be(expected_hash).unwrap();
        assert_eq!(
            hashed_data, expected_data,
            "Hashes do not match for oracle_name: {}, pair_id: {}, price: {}, timestamp: {}",
            oracle_name, pair_id, price, timestamp
        );
    }
}
