use serde::{Deserialize, Deserializer, Serialize};
use std::ops::RangeInclusive;
use utoipa::ToSchema;

#[derive(Debug)]
pub enum ConversionError {
    FailedSerialization,
    InvalidDateTime,
    BigDecimalConversion,
    FeltConversion,
    U128Conversion,
}

#[derive(Default, Debug, Serialize, Deserialize, ToSchema, Clone, Copy)]
pub enum AggregationMode {
    #[serde(rename = "median")]
    #[default]
    Median,
    #[serde(rename = "mean")]
    Mean,
    #[serde(rename = "twap")]
    Twap,
}

#[derive(Default, Debug, Serialize, Deserialize, ToSchema, Clone, Copy)]
pub enum Network {
    #[serde(rename = "testnet")]
    #[default]
    Testnet,
    #[serde(rename = "mainnet")]
    Mainnet,
}

#[derive(Default, Debug, Deserialize, ToSchema, Clone, Copy)]
pub enum DataType {
    #[serde(rename = "spot_entry")]
    #[default]
    SpotEntry,
    #[serde(rename = "perp_entry")]
    PerpEntry,
    #[serde(rename = "future_entry")]
    FutureEntry,
}

// Supported Aggregation Intervals
#[derive(Default, Debug, Serialize, Deserialize, ToSchema, Clone, Copy)]
pub enum Interval {
    #[serde(rename = "1min")]
    #[default]
    OneMinute,
    #[serde(rename = "15min")]
    FifteenMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "2h")]
    TwoHours,
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "1w")]
    OneWeek,
}

impl Interval {
    pub fn to_minutes(&self) -> i64 {
        match self {
            Interval::OneMinute => 1,
            Interval::FifteenMinutes => 15,
            Interval::OneHour => 60,
            Interval::TwoHours => 120,
            Interval::OneDay => 1400,
            Interval::OneWeek => 10080,
        }
    }

    pub fn to_seconds(&self) -> i64 {
        self.to_minutes() * 60
    }
}

#[derive(Debug, Clone)]
pub enum TimestampParam {
    Single(u64),
    Range(RangeInclusive<u64>),
}

impl From<u64> for TimestampParam {
    fn from(ts: u64) -> Self {
        TimestampParam::Single(ts)
    }
}

impl<'de> Deserialize<'de> for TimestampParam {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        if let Some((start, end)) = s.split_once(',') {
            let start = start.parse().map_err(serde::de::Error::custom)?;
            let end = end.parse().map_err(serde::de::Error::custom)?;
            Ok(TimestampParam::Range(start..=end))
        } else {
            let ts = s.parse().map_err(serde::de::Error::custom)?;
            Ok(TimestampParam::Single(ts))
        }
    }
}

pub fn deserialize_option_timestamp_param<'de, D>(
    deserializer: D,
) -> Result<Option<TimestampParam>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    if let Some(s) = s {
        if let Some((start, end)) = s.split_once(',') {
            let start = start.parse().map_err(serde::de::Error::custom)?;
            let end = end.parse().map_err(serde::de::Error::custom)?;
            Ok(Some(TimestampParam::Range(start..=end)))
        } else {
            let ts = s.parse().map_err(serde::de::Error::custom)?;
            Ok(Some(TimestampParam::Single(ts)))
        }
    } else {
        Ok(None)
    }
}
