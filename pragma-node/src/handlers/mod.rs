pub mod create_entry;
pub mod create_future_entry;
pub mod get_entry;
pub mod get_expiries;
pub mod get_ohlc;
pub mod get_volatility;
pub mod merkle_feeds;
pub mod onchain;
pub mod optimistic_oracle;
pub mod publish_entry_ws;
pub mod subscribe_to_entry;
pub mod subscribe_to_price;

pub use create_entry::create_entries;
pub use create_future_entry::create_future_entries;
pub use get_entry::get_entry;
pub use get_expiries::get_expiries;
pub use get_ohlc::get_ohlc;
pub use get_volatility::get_volatility;
pub use subscribe_to_entry::subscribe_to_entry;
pub use subscribe_to_price::subscribe_to_price;
pub use publish_entry_ws::publish_entry;

use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use pragma_common::types::{AggregationMode, DataType, Interval};

use crate::types::timestamp::UnixTimestamp;

#[derive(Default, Debug, Deserialize, ToSchema, Clone, Copy)]
pub enum EntryType {
    #[serde(rename = "spot")]
    #[default]
    Spot,
    #[serde(rename = "perp")]
    Perp,
    #[serde(rename = "future")]
    Future,
}

impl From<EntryType> for DataType {
    fn from(value: EntryType) -> Self {
        match value {
            EntryType::Spot => DataType::SpotEntry,
            EntryType::Future => DataType::FutureEntry,
            EntryType::Perp => DataType::PerpEntry,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct GetEntryParams {
    /// The unix timestamp in seconds. This endpoint will return the first update whose
    /// timestamp is <= the provided value.
    #[schema(value_type = i64)]
    pub timestamp: Option<UnixTimestamp>,
    pub interval: Option<Interval>,
    pub routing: Option<bool>,
    pub aggregation: Option<AggregationMode>,
    pub entry_type: Option<EntryType>,
    pub expiry: Option<String>,
}

impl Default for GetEntryParams {
    fn default() -> Self {
        Self {
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
            interval: Some(Interval::default()),
            routing: Some(false),
            aggregation: Some(AggregationMode::default()),
            entry_type: Some(EntryType::default()),
            expiry: None,
        }
    }
}
