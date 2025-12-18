use clickhouse::Row;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Simple price entry for ClickHouse
#[derive(Debug, Clone, PartialEq, Row, Serialize, Deserialize)]
pub(crate) struct PriceEntry {
    #[serde(with = "clickhouse::serde::uuid")]
    pub id: Uuid,
    pub pair_id: String,
    pub price: String,
    pub timestamp: u32,
    pub source: String,
}

/// Funding rate entry for ClickHouse
#[derive(Debug, Clone, PartialEq, Row, Serialize, Deserialize)]
pub(crate) struct FundingRateEntry {
    #[serde(with = "clickhouse::serde::uuid")]
    pub id: Uuid,
    pub pair_id: String,
    pub annualized_rate: f64,
    pub timestamp: u32,
    pub source: String,
}

/// Open interest entry for ClickHouse
#[derive(Debug, Clone, PartialEq, Row, Serialize, Deserialize)]
pub(crate) struct OpenInterestEntry {
    #[serde(with = "clickhouse::serde::uuid")]
    pub id: Uuid,
    pub pair_id: String,
    pub open_interest_value: f64,
    pub timestamp: u32,
    pub source: String,
}

/// Trade entry for ClickHouse
#[derive(Debug, Clone, PartialEq, Row, Serialize, Deserialize)]
pub(crate) struct TradeEntry {
    #[serde(with = "clickhouse::serde::uuid")]
    pub id: Uuid,
    pub pair_id: String,
    pub price: String,
    pub size: String,
    pub side: String,
    pub timestamp: u32,
    pub source: String,
    pub buyer_address: String,
    pub seller_address: String,
}
