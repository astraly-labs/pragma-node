use serde::{Deserialize, Serialize};

pub use create_entry::create_entries;
pub use get_entry::get_entry;
use starknet::core::types::FieldElement;
use utoipa::ToSchema;

pub mod create_entry;
pub mod get_entry;

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
    price: u128,
    timestamp: u64,
}
