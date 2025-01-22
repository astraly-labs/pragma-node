pub mod block_id;
pub mod merkle_tree;
pub mod options;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use utoipa::ToSchema;

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

#[derive(Default, Debug, Serialize, Deserialize, ToSchema, Clone, Copy, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum Network {
    #[default]
    #[serde(rename = "sepolia")]
    Sepolia,
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
#[derive(Default, Debug, Serialize, Deserialize, ToSchema, Clone, Copy, Eq, PartialEq, Hash)]
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
