use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;
use utoipa::ToSchema;


#[derive(Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct BaseEntry {
    pub timestamp: u64,
    pub source: String,
    pub publisher: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct Entry {
    pub base: BaseEntry,
    pub pair_id: String,
    pub price: u128,
    pub volume: u128,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateSpotRequest {
    pub signature: Vec<FieldElement>,
    pub pair_id: String,
    pub publisher_id: i32,
    pub data_range: CreateSpotTimestamp,
    pub price: u128,
    pub volume: u128,
    pub entries: Vec<Entry>
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct CreateSpotTimestamp {
    pub start_timestamp: u64,
    pub end_timestamp: u64,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct CreateSpotResponse {
    pub pair_id: String,
    pub price: u128,
    pub data_range: CreateSpotTimestamp,
    pub volume: u128,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePerpRequest {
    pub signature: Vec<FieldElement>,
    pub pair_id: String,
    pub publisher_id: i32,
    pub data_range: CreateSpotTimestamp,
    pub price: u128,
    pub open_interest: u128,
    pub funding_rate: u128,
    pub volume: u128,
    pub entries: Vec<Entry>
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct CreatePerpResponse {
    pub pair_id: String,
    pub price: u128,
    pub basis: u128,
    pub open_interest: u128,
    pub funding_rate: u128,
    pub data_range: CreateSpotTimestamp,
    pub volume: u128,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateEntryRequest {
    pub signature: Vec<FieldElement>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateEntryResponse {
    pub number_entries_created: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetEntryResponse {
    pub num_sources_aggregated: usize,
    pub pair_id: String,
    pub price: String,
    pub timestamp: u64,
    pub decimals: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetVolatilityResponse {
    pub pair_id: String,
    pub volatility: f64,
    pub decimals: u32,
}
