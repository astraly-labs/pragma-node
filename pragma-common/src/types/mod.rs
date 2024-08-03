pub mod instrument;
pub mod merkle_tree;

use core::fmt;
use serde::{Deserialize, Serialize};
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

#[derive(Default, Debug, Serialize, Deserialize, ToSchema, Clone, Copy)]
pub enum Network {
    #[serde(rename = "sepolia")]
    #[default]
    Sepolia,
    #[serde(rename = "mainnet")]
    Mainnet,
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        match self {
            Network::Sepolia => write!(f, "sepolia"),
            Network::Mainnet => write!(f, "mainnet"),
        }
    }
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
