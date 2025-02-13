use std::str::FromStr;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use starknet::core::{
    crypto::compute_hash_on_elements, types::Felt, utils::cairo_short_string_to_felt,
};
use strum::{Display, EnumString};
use thiserror::Error;
use utoipa::ToSchema;

use crate::utils::field_element_as_hex_string;

/// The available currencies supported.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Display, EnumString, ToSchema)]
#[strum(serialize_all = "UPPERCASE")]
pub enum OptionCurrency {
    BTC,
    ETH,
}

impl OptionCurrency {
    pub fn from_ticker(ticker: &str) -> Result<Self, InstrumentError> {
        ticker
            .parse()
            .map_err(|_| InstrumentError::UnsupportedCurrency(ticker.to_owned()))
    }
}

/// The possible types for an option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, EnumString, ToSchema)]
#[strum(serialize_all = "UPPERCASE")]
pub enum OptionType {
    #[strum(serialize = "P")]
    Put,
    #[strum(serialize = "C")]
    Call,
}

#[derive(Debug, Error)]
pub enum InstrumentError {
    #[error("invalid name format: {0}")]
    NameFormat(String),
    #[error("invalid date format: {0}")]
    DateFormat(String),
    #[error("invalid option type: {0}")]
    OptionType(String),
    #[error("invalid mark price: {0}")]
    MarkPrice(String),
    #[error("currency must be BTC or ETH, found: {0}")]
    UnsupportedCurrency(String),
    #[error("could not convert {0} to a field element")]
    Felt(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
/// An instrument.
pub struct Instrument {
    pub base_currency: OptionCurrency,
    pub expiration_date: NaiveDate,
    #[schema(value_type = u64)]
    pub strike_price: BigDecimal,
    pub option_type: OptionType,
}

impl Instrument {
    pub fn from_name(instrument_name: &str) -> Result<Self, InstrumentError> {
        let parts: Vec<&str> = instrument_name.split('-').collect();

        if parts.len() != 4 {
            return Err(InstrumentError::NameFormat(instrument_name.to_owned()));
        }

        let base_currency = OptionCurrency::from_ticker(parts[0])?;
        let expiration_date = NaiveDate::parse_from_str(parts[1], "%d%b%y")
            .map_err(|_| InstrumentError::DateFormat(parts[1].to_owned()))?;
        let strike_price = BigDecimal::from_str(parts[2])
            .map_err(|_| InstrumentError::MarkPrice(parts[2].to_owned()))?;
        let option_type = match parts.get(3) {
            Some(&"P") => OptionType::Put,
            Some(&"C") => OptionType::Call,
            _ => return Err(InstrumentError::OptionType(parts[3].to_owned())),
        };

        Ok(Self {
            base_currency,
            expiration_date,
            strike_price,
            option_type,
        })
    }

    pub fn name(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            self.base_currency,
            self.expiration_date
                .format("%d%b%y")
                .to_string()
                .to_uppercase(),
            self.strike_price,
            self.option_type
        )
    }
}

#[macro_export]
macro_rules! instrument {
    ($($name:expr),* $(,)?) => {$(
        {
            const _: () = {
                let s = $name;
                assert!(s.len() >= 11, "Instrument name too short");
                assert!(s.as_bytes()[3] == b'-' && s.as_bytes()[11] == b'-' && s.as_bytes()[s.len() - 2] == b'-', "Invalid format");
            };
            Instrument::from_name($name).expect(&format!("Could not use macro instrument! from: {}", $name))
        }
    )*};
}

/// An instrument option with its mark price for a certain timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OptionData {
    pub instrument_name: String,
    pub base_currency: OptionCurrency,
    pub current_timestamp: i64,
    #[schema(value_type = u64)]
    pub mark_price: BigDecimal,
}

impl OptionData {
    /// Converts an option as a Vec of Felt - i.e a calldata.
    pub fn as_calldata(&self) -> Result<Vec<Felt>, InstrumentError> {
        Ok(vec![
            cairo_short_string_to_felt(&self.instrument_name)
                .map_err(|_| InstrumentError::Felt("instrument name".to_string()))?,
            cairo_short_string_to_felt(&self.base_currency.to_string())
                .map_err(|_| InstrumentError::Felt("base currency".to_string()))?,
            Felt::from(self.current_timestamp as u64),
            Felt::from_str(&self.mark_price.to_string())
                .map_err(|_| InstrumentError::Felt("mark price".to_string()))?,
        ])
    }

    /// Computes the pedersen hash of the Option.
    pub fn pedersen_hash(&self) -> Result<Felt, InstrumentError> {
        let elements = self.as_calldata()?;
        Ok(compute_hash_on_elements(&elements))
    }

    pub fn pedersen_hash_as_hex_string(&self) -> Result<String, InstrumentError> {
        let hash = self.pedersen_hash()?;
        Ok(field_element_as_hex_string(&hash))
    }
}

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use rstest::rstest;
    use std::str::FromStr;

    use super::*;

    #[rstest]
    #[case(
        "BTC-27JUN25-80000-P",
        OptionCurrency::BTC,
        2025,
        6,
        27,
        80_000,
        OptionType::Put
    )]
    #[case(
        "BTC-16AUG24-59000-P",
        OptionCurrency::BTC,
        2024,
        8,
        16,
        59_000,
        OptionType::Put
    )]
    #[case(
        "BTC-16AUG24-54000-C",
        OptionCurrency::BTC,
        2024,
        8,
        16,
        54_000,
        OptionType::Call
    )]
    #[case(
        "BTC-27DEC24-20000-P",
        OptionCurrency::BTC,
        2024,
        12,
        27,
        20_000,
        OptionType::Put
    )]
    #[case(
        "BTC-3AUG24-61500-C",
        OptionCurrency::BTC,
        2024,
        8,
        3,
        61_500,
        OptionType::Call
    )]
    #[case(
        "BTC-27DEC24-28000-P",
        OptionCurrency::BTC,
        2024,
        12,
        27,
        28_000,
        OptionType::Put
    )]
    #[case(
        "BTC-3AUG24-61000-P",
        OptionCurrency::BTC,
        2024,
        8,
        3,
        61_000,
        OptionType::Put
    )]
    #[case(
        "BTC-30AUG24-78000-P",
        OptionCurrency::BTC,
        2024,
        8,
        30,
        78_000,
        OptionType::Put
    )]
    #[case(
        "BTC-27DEC24-105000-C",
        OptionCurrency::BTC,
        2024,
        12,
        27,
        105_000,
        OptionType::Call
    )]
    #[case(
        "BTC-4AUG24-56000-P",
        OptionCurrency::BTC,
        2024,
        8,
        4,
        56_000,
        OptionType::Put
    )]
    fn test_instrument_from_name(
        #[case] name: &str,
        #[case] expected_currency: OptionCurrency,
        #[case] expected_year: i32,
        #[case] expected_month: u32,
        #[case] expected_day: u32,
        #[case] expected_strike: i32,
        #[case] expected_option_type: OptionType,
    ) {
        let instrument = Instrument::from_name(name).unwrap();
        assert_eq!(instrument.base_currency, expected_currency);
        assert_eq!(
            instrument.expiration_date,
            NaiveDate::from_ymd_opt(expected_year, expected_month, expected_day).unwrap()
        );
        assert_eq!(
            instrument.strike_price,
            BigDecimal::from_str(&expected_strike.to_string()).unwrap()
        );
        assert_eq!(instrument.option_type, expected_option_type);
    }

    #[rstest]
    #[case("BTC-27JUN25-80000-X")]
    #[case("BTC-16AUG24-59000")]
    #[case("BTC-16AUG24-ABCDE-C")]
    #[case("-27DEC24")]
    #[case("BTC-3AUG24-61500-C-EXTRA")]
    #[case("INVALID-27DEC24-28000-P")]
    #[case("SOL-4AUG24-56000-P")]
    #[case("ETH-2424AUG24-56000-P")]
    fn test_invalid_instrument_names(#[case] name: &str) {
        assert!(Instrument::from_name(name).is_err());
    }
}
