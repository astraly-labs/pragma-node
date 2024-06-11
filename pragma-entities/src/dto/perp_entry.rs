use bigdecimal::ToPrimitive;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, ToSchema)]
pub struct PerpEntry {
    pub id: Uuid,
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: u64,
    pub expiration_timestamp: Option<u64>,
    pub publisher_signature: String,
    pub price: u128,
}

#[derive(Deserialize)]
#[allow(unused)]
pub struct PerpEntriesFilter {
    pub(crate) pair_id: Option<String>,
    pub(crate) publisher_contains: Option<String>,
}

impl From<crate::PerpEntry> for PerpEntry {
    fn from(perp_entry: crate::PerpEntry) -> Self {
        Self {
            id: perp_entry.id,
            pair_id: perp_entry.pair_id,
            publisher: perp_entry.publisher,
            source: perp_entry.source,
            timestamp: perp_entry.timestamp.and_utc().timestamp_millis() as u64,
            expiration_timestamp: Option::None,
            publisher_signature: perp_entry.publisher_signature,
            price: perp_entry.price.to_u128().unwrap_or(0), // change default value ?
        }
    }
}
