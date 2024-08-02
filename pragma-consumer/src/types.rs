use std::str::FromStr;

use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};

pub type MerkleProof = Vec<u64>;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct InstrumentName(String);

pub enum OptionType {
    Put,
    Call,
}

impl OptionType {
    pub fn to_str(&self) -> &str {
        match self {
            OptionType::Put => "P",
            OptionType::Call => "C",
        }
    }
}

#[macro_export]
macro_rules! instrument {
    ($name:expr) => {
        Instrument::from_name($name).expect("Failed to create instrument")
    };
}

pub struct Instrument {
    pub asset: String,
    pub expiration_date: DateTime<Utc>,
    pub strike_price: BigDecimal,
    pub option_type: OptionType,
}

impl Instrument {
    pub fn from_name(instrument_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = instrument_name.split('-').collect();

        if parts.len() != 4 {
            return Err("Invalid instrument name format".into());
        }

        let asset = parts[0].to_string();

        let expiration_date = NaiveDate::parse_from_str(parts[1], "%d%b%y")?
            .and_hms_opt(0, 0, 0)
            .ok_or("Invalid time")?
            .and_local_timezone(Utc)
            .single()
            .ok_or("Ambiguous timezone")?;

        let strike_price = BigDecimal::from_str(parts[2])?;

        let option_type = match parts[3] {
            "P" => OptionType::Put,
            "C" => OptionType::Call,
            _ => return Err("Invalid option type".into()),
        };

        Ok(Instrument {
            asset,
            expiration_date,
            strike_price,
            option_type,
        })
    }

    pub fn name(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            self.asset,
            self.expiration_date.timestamp(),
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
