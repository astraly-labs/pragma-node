use std::str::FromStr;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use color_eyre::{eyre::eyre, Result};

pub type MerkleProof = Vec<u64>;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct InstrumentName(String);

#[derive(Debug, PartialEq)]
pub enum OptionType {
    Put,
    Call,
}

impl OptionType {
    pub fn to_str(&self) -> &str {
        match self {
            Self::Put => "P",
            Self::Call => "C",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Currency {
    BTC,
    ETH,
}

impl Currency {
    pub fn to_str(&self) -> &str {
        match self {
            Self::BTC => "BTC",
            Self::ETH => "ETH",
        }
    }
}

impl Currency {
    pub fn from_ticker(ticker: &str) -> Result<Self> {
        let currency = match ticker {
            "BTC" => Self::BTC,
            "ETH" => Self::ETH,
            _ => return Err(eyre!("Invalid currency")),
        };
        Ok(currency)
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

pub struct Instrument {
    pub base_currency: Currency,
    pub expiration_date: NaiveDate,
    pub strike_price: BigDecimal,
    pub option_type: OptionType,
}

impl Instrument {
    pub fn from_name(instrument_name: &str) -> Result<Self> {
        let parts: Vec<&str> = instrument_name.split('-').collect();

        if parts.len() != 4 {
            return Err(eyre!("Invalid instrument name format"));
        }

        let base_currency = Currency::from_ticker(parts[0])?;

        let expiration_date = NaiveDate::parse_from_str(parts[1], "%d%b%y")?;

        let strike_price = BigDecimal::from_str(parts[2])?;

        let option_type = match parts[3] {
            "P" => OptionType::Put,
            "C" => OptionType::Call,
            _ => return Err(eyre!("Invalid option type")),
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
            self.base_currency.to_str(),
            self.expiration_date,
            self.strike_price,
            self.option_type.to_str()
        )
    }
}

#[derive(Default, Debug)]
pub struct OptionData {
    pub instrument_name: String,
    pub base_currency: String,
    pub current_timestamp: i64,
    pub mark_price: BigDecimal,
}

#[derive(Default, Debug)]
pub struct MerkleFeedCalldata {
    pub merkle_proof: MerkleProof,
    pub option_data: OptionData,
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
        Currency::BTC,
        2025,
        6,
        27,
        80000,
        OptionType::Put
    )]
    #[case(
        "BTC-16AUG24-59000-P",
        Currency::BTC,
        2024,
        8,
        16,
        59000,
        OptionType::Put
    )]
    #[case(
        "BTC-16AUG24-54000-C",
        Currency::BTC,
        2024,
        8,
        16,
        54000,
        OptionType::Call
    )]
    #[case(
        "BTC-27DEC24-20000-P",
        Currency::BTC,
        2024,
        12,
        27,
        20000,
        OptionType::Put
    )]
    #[case(
        "BTC-3AUG24-61500-C",
        Currency::BTC,
        2024,
        8,
        3,
        61500,
        OptionType::Call
    )]
    #[case(
        "BTC-27DEC24-28000-P",
        Currency::BTC,
        2024,
        12,
        27,
        28000,
        OptionType::Put
    )]
    #[case(
        "BTC-3AUG24-61000-P",
        Currency::BTC,
        2024,
        8,
        3,
        61000,
        OptionType::Put
    )]
    #[case(
        "BTC-30AUG24-78000-P",
        Currency::BTC,
        2024,
        8,
        30,
        78000,
        OptionType::Put
    )]
    #[case(
        "BTC-27DEC24-105000-C",
        Currency::BTC,
        2024,
        12,
        27,
        105000,
        OptionType::Call
    )]
    #[case(
        "BTC-4AUG24-56000-P",
        Currency::BTC,
        2024,
        8,
        4,
        56000,
        OptionType::Put
    )]
    fn test_instrument_from_name(
        #[case] name: &str,
        #[case] expected_currency: Currency,
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
