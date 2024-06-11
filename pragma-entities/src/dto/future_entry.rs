use bigdecimal::ToPrimitive;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, ToSchema)]
pub struct FutureEntry {
    pub id: Uuid,
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: u64,
    pub expiration_timestamp: u64,
    pub publisher_signature: String,
    pub price: u128,
}

#[derive(Deserialize)]
#[allow(unused)]
pub struct FutureEntriesFilter {
    pub(crate) pair_id: Option<String>,
    pub(crate) publisher_contains: Option<String>,
}

impl From<crate::FutureEntry> for FutureEntry {
    fn from(future_entry: crate::FutureEntry) -> Self {
        Self {
            id: future_entry.id,
            pair_id: future_entry.pair_id,
            publisher: future_entry.publisher,
            source: future_entry.source,
            timestamp: future_entry.timestamp.and_utc().timestamp_millis() as u64,
            expiration_timestamp: future_entry
                .expiration_timestamp
                .and_utc()
                .timestamp_millis() as u64,
            publisher_signature: future_entry.publisher_signature,
            price: future_entry.price.to_u128().unwrap_or(0), // change default value ?
        }
    }
}
