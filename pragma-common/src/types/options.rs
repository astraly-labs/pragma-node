use std::str::FromStr;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use starknet::core::{
    crypto::compute_hash_on_elements, types::FieldElement, utils::cairo_short_string_to_felt,
};
use thiserror::Error;

/// The available currencies supported.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum OptionCurrency {
    BTC,
    ETH,
}

impl OptionCurrency {
    pub fn as_str(&self) -> &str {
        match self {
            Self::BTC => "BTC",
            Self::ETH => "ETH",
        }
    }
}

impl OptionCurrency {
    pub fn from_ticker(ticker: &str) -> Result<Self, InstrumentError> {
        let currency = match ticker.to_uppercase().as_str() {
            "BTC" => Self::BTC,
            "ETH" => Self::ETH,
            _ => return Err(InstrumentError::UnsupportedCurrency(ticker.to_owned())),
        };
        Ok(currency)
    }
}

/// The possible types for an option.
#[derive(Debug, PartialEq)]
pub enum OptionType {
    Put,
    Call,
}

impl OptionType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Put => "P",
            Self::Call => "C",
        }
    }
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
}

/// An instrument.
pub struct Instrument {
    pub base_currency: OptionCurrency,
    pub expiration_date: NaiveDate,
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
        let option_type = match parts[3] {
            "P" => OptionType::Put,
            "C" => OptionType::Call,
            _ => return Err(InstrumentError::OptionType(parts[3].to_owned())),
        };

        Ok(Instrument {
            base_currency,
            expiration_date,
            strike_price,
            option_type,
        })
    }

    pub fn name(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            self.base_currency.as_str(),
            self.expiration_date,
            self.strike_price,
            self.option_type.as_str()
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
#[derive(Debug, Serialize, Deserialize)]
pub struct OptionData {
    pub instrument_name: String,
    pub base_currency: OptionCurrency,
    pub current_timestamp: i64,
    pub mark_price: BigDecimal,
}

impl OptionData {
    pub fn pedersen_hash(&self) -> FieldElement {
        // TODO(akhercha): Handle unwraps
        let elements: Vec<FieldElement> = vec![
            cairo_short_string_to_felt(&self.instrument_name).unwrap(),
            cairo_short_string_to_felt(self.base_currency.as_str()).unwrap(),
            FieldElement::from(self.current_timestamp as u64),
            FieldElement::from_str(&self.mark_price.to_string()).unwrap(),
        ];
        compute_hash_on_elements(&elements)
    }

    pub fn hexadecimal_hash(&self) -> String {
        let hash = self.pedersen_hash();
        format!("0x{:x}", hash)
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
        80000,
        OptionType::Put
    )]
    #[case(
        "BTC-16AUG24-59000-P",
        OptionCurrency::BTC,
        2024,
        8,
        16,
        59000,
        OptionType::Put
    )]
    #[case(
        "BTC-16AUG24-54000-C",
        OptionCurrency::BTC,
        2024,
        8,
        16,
        54000,
        OptionType::Call
    )]
    #[case(
        "BTC-27DEC24-20000-P",
        OptionCurrency::BTC,
        2024,
        12,
        27,
        20000,
        OptionType::Put
    )]
    #[case(
        "BTC-3AUG24-61500-C",
        OptionCurrency::BTC,
        2024,
        8,
        3,
        61500,
        OptionType::Call
    )]
    #[case(
        "BTC-27DEC24-28000-P",
        OptionCurrency::BTC,
        2024,
        12,
        27,
        28000,
        OptionType::Put
    )]
    #[case(
        "BTC-3AUG24-61000-P",
        OptionCurrency::BTC,
        2024,
        8,
        3,
        61000,
        OptionType::Put
    )]
    #[case(
        "BTC-30AUG24-78000-P",
        OptionCurrency::BTC,
        2024,
        8,
        30,
        78000,
        OptionType::Put
    )]
    #[case(
        "BTC-27DEC24-105000-C",
        OptionCurrency::BTC,
        2024,
        12,
        27,
        105000,
        OptionType::Call
    )]
    #[case(
        "BTC-4AUG24-56000-P",
        OptionCurrency::BTC,
        2024,
        8,
        4,
        56000,
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
