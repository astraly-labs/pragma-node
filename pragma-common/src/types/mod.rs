pub mod auth;
pub mod block_id;
pub mod entries;
pub mod hex_hash;
pub mod merkle_tree;
pub mod options;
pub mod pair;
pub mod timestamp;
pub mod typed_data;
pub mod utils;

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

#[derive(Default, Debug, Deserialize, ToSchema, Clone, Copy, PartialEq, Eq)]
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
    #[serde(rename = "1s")]
    OneSecond,
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
    pub const fn to_minutes(&self) -> i64 {
        match self {
            Self::OneSecond => 0,
            Self::OneMinute => 1,
            Self::FifteenMinutes => 15,
            Self::OneHour => 60,
            Self::TwoHours => 120,
            Self::OneDay => 1400,
            Self::OneWeek => 10080,
        }
    }

    pub const fn to_seconds(&self) -> i64 {
        if matches!(self, Self::OneSecond) {
            return 1;
        }
        self.to_minutes() * 60
    }
}
