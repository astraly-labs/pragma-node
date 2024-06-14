use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;
use utoipa::{IntoParams, ToSchema};

use pragma_common::types::{AggregationMode, DataType, Interval, Network};

pub use create_entry::create_entries;
pub use get_entry::get_entry;
pub use get_ohlc::get_ohlc;
pub use get_onchain::get_onchain;
pub use get_volatility::get_volatility;

use crate::{
    infra::repositories::entry_repository::OHLCEntry,
    utils::{doc_examples, UnixTimestamp},
};

pub mod create_entry;
pub mod get_entry;
pub mod get_ohlc;
pub mod get_onchain;
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

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct GetOnchainParams {
    pub network: Network,
    pub aggregation: Option<AggregationMode>,
    pub timestamp: Option<u64>,
}

impl Default for GetOnchainParams {
    fn default() -> Self {
        Self {
            network: Network::default(),
            aggregation: None,
            timestamp: Some(chrono::Utc::now().timestamp() as u64),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OnchainEntry {
    pub publisher: String,
    pub source: String,
    pub price: String,
    pub tx_hash: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetOnchainResponse {
    pair_id: String,
    last_updated_timestamp: u64,
    price: String,
    decimals: u32,
    nb_sources_aggregated: u32,
    asset_type: String,
    components: Vec<OnchainEntry>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct GetOnchainCheckpointsParams {
    pub network: Network,
    pub limit: Option<u64>,
}

impl Default for GetOnchainCheckpointsParams {
    fn default() -> Self {
        Self {
            network: Network::default(),
            limit: Some(100),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Checkpoint {
    pub tx_hash: String,
    pub price: String,
    pub timestamp: u64,
    pub sender_address: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetOnchainCheckpointsResponse(pub Vec<Checkpoint>);

/// Query parameters structs

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct GetEntryParams {
    /// The unix timestamp in seconds. This endpoint will return the first update whose
    /// timestamp is <= the provided value.
    #[param(value_type = i64)]
    #[param(example = doc_examples::timestamp_example)]
    pub timestamp: Option<UnixTimestamp>,
    pub interval: Option<Interval>,
    pub routing: Option<bool>,
    pub aggregation: Option<AggregationMode>,
}

impl Default for GetEntryParams {
    fn default() -> Self {
        Self {
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
            interval: Some(Interval::default()),
            routing: Some(false),
            aggregation: Some(AggregationMode::default()),
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
pub struct GetOnchainPublishersParams {
    pub network: Network,
    pub data_type: DataType,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PublisherEntry {
    pub pair_id: String,
    pub last_updated_timestamp: u64,
    pub price: String,
    pub source: String,
    pub decimals: u32,
    pub daily_updates: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Publisher {
    pub publisher: String,
    pub website_url: String,
    pub last_updated_timestamp: u64,
    pub r#type: u32,
    pub nb_feeds: u32,
    pub daily_updates: u32,
    pub total_updates: u32,
    pub components: Vec<PublisherEntry>,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct GetOnchainPublishersResponse(pub Vec<Publisher>);

#[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
pub struct GetOnchainOHLCParams {
    pub network: Network,
    pub interval: Interval,
    pub limit: Option<u64>,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct GetOnchainOHLCResponse {
    pub pair_id: String,
    pub data: Vec<OHLCEntry>,
}
