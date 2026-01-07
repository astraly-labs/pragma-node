use clickhouse::Row;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Price entry for ClickHouse with unified market_id format
#[derive(Debug, Clone, PartialEq, Row, Serialize, Deserialize)]
pub(crate) struct PriceEntry {
    #[serde(with = "clickhouse::serde::uuid")]
    pub id: Uuid,
    pub market_id: String,       // Unified format: BASE:QUOTE:TYPE
    pub instrument_type: String, // SPOT or PERP
    pub pair_id: String,         // Legacy format: BASE/QUOTE
    pub price: String,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub exchange_timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub received_timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
}

/// Funding rate entry for ClickHouse with unified market_id format
#[derive(Debug, Clone, PartialEq, Row, Serialize, Deserialize)]
pub(crate) struct FundingRateEntry {
    #[serde(with = "clickhouse::serde::uuid")]
    pub id: Uuid,
    pub market_id: String,       // Unified format: BASE:QUOTE:TYPE
    pub instrument_type: String, // SPOT or PERP
    pub pair_id: String,         // Legacy format: BASE/QUOTE
    pub annualized_rate: f64,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub exchange_timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub received_timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
}

/// Open interest entry for ClickHouse with unified market_id format
#[derive(Debug, Clone, PartialEq, Row, Serialize, Deserialize)]
pub(crate) struct OpenInterestEntry {
    #[serde(with = "clickhouse::serde::uuid")]
    pub id: Uuid,
    pub market_id: String,       // Unified format: BASE:QUOTE:TYPE
    pub instrument_type: String, // SPOT or PERP
    pub pair_id: String,         // Legacy format: BASE/QUOTE
    pub open_interest_value: f64,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub exchange_timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub received_timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
}

/// Trade entry for ClickHouse with unified market_id format
#[derive(Debug, Clone, PartialEq, Row, Serialize, Deserialize)]
pub(crate) struct TradeEntry {
    #[serde(with = "clickhouse::serde::uuid")]
    pub id: Uuid,
    pub market_id: String,       // Unified format: BASE:QUOTE:TYPE
    pub instrument_type: String, // SPOT or PERP
    pub pair_id: String,         // Legacy format: BASE/QUOTE
    pub price: String,
    pub size: String,
    pub side: String,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub exchange_timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub received_timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
    pub buyer_address: String,
    pub seller_address: String,
}

/// Helper to create market_id from pair and instrument type
/// Format: BASE:QUOTE:TYPE (e.g., BTC:USD:PERP)
pub(crate) fn make_market_id(
    pair: &pragma_common::Pair,
    instrument_type: pragma_common::InstrumentType,
) -> String {
    let type_str = match instrument_type {
        pragma_common::InstrumentType::Spot => "SPOT",
        pragma_common::InstrumentType::Perp => "PERP",
    };
    format!("{}:{}:{}", pair.base, pair.quote, type_str)
}

/// Helper to convert InstrumentType to string
pub(crate) fn instrument_type_str(instrument_type: pragma_common::InstrumentType) -> String {
    match instrument_type {
        pragma_common::InstrumentType::Spot => "SPOT".to_string(),
        pragma_common::InstrumentType::Perp => "PERP".to_string(),
    }
}
