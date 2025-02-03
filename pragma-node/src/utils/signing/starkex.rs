use bigdecimal::{BigDecimal, ToPrimitive};
use pragma_common::errors::ConversionError;
use starknet::core::{crypto::pedersen_hash, types::Felt, utils::cairo_short_string_to_felt};

use super::Signable;

pub struct StarkexPrice {
    pub oracle_name: String,
    pub pair_id: String,
    pub timestamp: u64,
    pub price: BigDecimal,
}

impl StarkexPrice {
    pub fn get_global_asset_id(pair_id: &str) -> Result<String, ConversionError> {
        let pair_id = pair_id.replace('/', "-");
        let pair_id = if !pair_id.contains('-') {
            let (first, second) = pair_id.split_at(3);
            format!("{}-{}-8", first, second)
        } else {
            format!("{}-8", pair_id)
        };

        let felt =
            cairo_short_string_to_felt(&pair_id).map_err(|_| ConversionError::FeltConversion)?;
        let hex = format!("{:x}", felt);
        Ok(format!("{:0<30}", hex))
    }

    pub fn get_oracle_asset_id(
        oracle_name: &str,
        pair_id: &str,
    ) -> Result<String, ConversionError> {
        let market_name = pair_id.replace(['/', '-'], "");

        let market_felt = cairo_short_string_to_felt(&market_name)
            .map_err(|_| ConversionError::FeltConversion)?;
        let oracle_felt =
            cairo_short_string_to_felt(oracle_name).map_err(|_| ConversionError::FeltConversion)?;

        let market_hex = format!("{:x}", market_felt);
        let oracle_hex = format!("{:x}", oracle_felt);

        Ok(format!("{:0<32}{:0<8}00", market_hex, oracle_hex))
    }

    /// Builds the first number for the hash computation based on oracle name and pair id.
    pub fn build_external_asset_id(
        oracle_name: &str,
        pair_id: &str,
    ) -> Result<Felt, ConversionError> {
        let external_asset_id = Self::get_oracle_asset_id(oracle_name, pair_id)?;
        Felt::from_hex(&external_asset_id).map_err(|_| ConversionError::FeltConversion)
    }

    /// Builds the second number for the hash computation based on timestamp and price.
    pub fn build_second_number(
        timestamp: u128,
        price: &BigDecimal,
    ) -> Result<Felt, ConversionError> {
        let price = price.to_u128().ok_or(ConversionError::U128Conversion)?;
        let price_as_hex = format!("{:x}", price);
        let timestamp_as_hex = format!("{:x}", timestamp);
        let v = format!("{}{}", price_as_hex, timestamp_as_hex);
        Felt::from_hex(&v).map_err(|_| ConversionError::FeltConversion)
    }
}

impl Signable for StarkexPrice {
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
    fn try_get_hash(&self) -> Result<Felt, ConversionError> {
        let first_number = Self::build_external_asset_id(&self.oracle_name, &self.pair_id)?;
        let second_number = Self::build_second_number(self.timestamp as u128, &self.price)?;
        Ok(pedersen_hash(&first_number, &second_number))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use std::str::FromStr;

    use super::*;
    use bigdecimal::BigDecimal;

    #[rstest]
    #[case("BTC-USD", "4254432d5553442d38000000000000")]
    #[case("EUR-USD", "4555522d5553442d38000000000000")]
    #[case("BTC/USD", "4254432d5553442d38000000000000")]
    #[case("BTCUSD", "4254432d5553442d38000000000000")]
    fn test_get_encoded_pair_id(#[case] pair_id: &str, #[case] expected_encoded_pair_id: &str) {
        let encoded_pair_id =
            StarkexPrice::get_global_asset_id(pair_id).expect("Could not encode pair id");
        assert_eq!(
            encoded_pair_id.to_lowercase(),
            expected_encoded_pair_id.to_lowercase(),
            "Encoded pair id does not match for pair_id: {}",
            pair_id
        );
    }

    #[rstest]
    #[case("PRGM", "BTC-USD", "425443555344000000000000000000005052474d00")]
    #[case("PRGM", "EUR-USD", "455552555344000000000000000000005052474d00")]
    #[case("PRGM", "BTC/USD", "425443555344000000000000000000005052474d00")]
    #[case("PRGM", "BTCUSD", "425443555344000000000000000000005052474d00")]
    fn test_get_oracle_asset_id(
        #[case] oracle_name: &str,
        #[case] pair_id: &str,
        #[case] expected_id: &str,
    ) {
        let oracle_asset_id = StarkexPrice::get_oracle_asset_id(oracle_name, pair_id)
            .expect("Could not get oracle asset id");
        assert_eq!(
            oracle_asset_id.to_lowercase(),
            expected_id.to_lowercase(),
            "Oracle asset id does not match for oracle: {}, pair: {}",
            oracle_name,
            pair_id
        );
    }

    #[rstest]
    #[case(
        "PRGM",
        "SOLUSD",
        "19511280076",
        1577216800,
        "230d86465a37eaa5221191bc196a86c2fc941e6c573322f24710b165285d23c"
    )]
    #[case(
        "PRGM",
        "ETHUSD",
        "369511280076",
        1577816800,
        "3e87426d2b40470cd314071d1dc93adf59e6906d40b85ad5e0f0c926b49d5f4"
    )]
    #[case(
        "TEST",
        "DOGEUSD",
        "51128006",
        1517816800,
        "65de8d73f0359a73e79c6b7f1ffe708159d378cc3da8edd308c92eaf8288d1c"
    )]
    #[case(
        "TEST",
        "DOGE/USD",
        "51128006",
        1517816800,
        "65de8d73f0359a73e79c6b7f1ffe708159d378cc3da8edd308c92eaf8288d1c"
    )]
    fn test_get_entry_hash(
        #[case] oracle_name: &str,
        #[case] pair_id: &str,
        #[case] price: &str,
        #[case] timestamp: u64,
        #[case] expected_hash: &str,
    ) {
        let price = BigDecimal::from_str(price).unwrap();
        let starkex_price = StarkexPrice {
            oracle_name: oracle_name.to_string(),
            pair_id: pair_id.to_string(),
            timestamp,
            price: price.clone(),
        };
        let hashed_data = starkex_price.try_get_hash().expect("Could not build hash");
        let expected_data = Felt::from_hex(expected_hash).unwrap();
        assert_eq!(
            hashed_data, expected_data,
            "Hashes do not match for oracle_name: {}, pair_id: {}, price: {}, timestamp: {}",
            oracle_name, pair_id, price, timestamp
        );
    }
}
