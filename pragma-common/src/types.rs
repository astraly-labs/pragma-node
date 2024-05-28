use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Default, Debug, Deserialize, ToSchema, Clone, Copy)]
pub enum AggregationMode {
    #[serde(rename = "median")]
    #[default]
    Median,
    #[serde(rename = "mean")]
    Mean,
    #[serde(rename = "twap")]
    Twap,
}

// Supported Aggregation Intervals
#[derive(Default, Debug, Deserialize, ToSchema, Clone, Copy)]
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
}

#[derive(Default, Debug, Deserialize, ToSchema, Clone, Copy)]
pub enum Network {
    #[serde(rename = "testnet")]
    #[default]
    Testnet,
    #[serde(rename = "mainnet")]
    Mainnet,
}
