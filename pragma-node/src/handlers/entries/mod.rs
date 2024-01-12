use serde::{Deserialize, Serialize};

use starknet::core::types::FieldElement;
use utoipa::{IntoParams, ToSchema};

pub use create_entry::create_entries;
pub use get_entry::get_entry;
pub use get_ohlc::get_ohlc;
pub use get_volatility::get_volatility;

use crate::infra::repositories::entry_repository::OHLCEntry;

pub mod create_entry;
pub mod get_entry;
pub mod get_ohlc;
pub mod get_volatility;

pub mod utils;

#[derive(Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct BaseEntry {
    timestamp: u64,
    source: String,
    publisher: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct Entry {
    base: BaseEntry,
    pair_id: String,
    price: u128,
    volume: u128,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateEntryRequest {
    signature: Vec<FieldElement>,
    entries: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateEntryResponse {
    number_entries_created: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetEntryResponse {
    num_sources_aggregated: usize,
    pair_id: String,
    price: String,
    timestamp: u64,
    decimals: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetOHLCResponse {
    pair_id: String,
    data: Vec<OHLCEntry>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetVolatilityResponse {
    pair_id: String,
    volatility: f64,
    decimals: u32,
}

/// Query parameters structs

// Define an enum for the allowed intervals
#[derive(Default, Debug, Deserialize, ToSchema)]
pub enum Interval {
    #[serde(rename = "1min")]
    #[default]
    OneMinute,
    #[serde(rename = "15min")]
    FifteenMinutes,
    #[serde(rename = "1h")]
    OneHour,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct GetEntryParams {
    pub timestamp: Option<u64>,
    pub interval: Option<Interval>,
}

impl Default for GetEntryParams {
    fn default() -> Self {
        Self {
            timestamp: Some(chrono::Utc::now().timestamp_millis() as u64),
            interval: Some(Interval::default()),
        }
    }
}
