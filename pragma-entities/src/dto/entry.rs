use bigdecimal::ToPrimitive;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, ToSchema)]
pub struct Entry {
    pub id: Uuid,
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: u64,
    pub publisher_signature: Option<String>,
    pub price: u128,
}

#[derive(Deserialize)]
#[allow(unused)]
pub struct EntriesFilter {
    pub(crate) pair_id: Option<String>,
    pub(crate) publisher_contains: Option<String>,
}

impl From<crate::Entry> for Entry {
    fn from(entry: crate::Entry) -> Self {
        Self {
            id: entry.id,
            pair_id: entry.pair_id,
            publisher: entry.publisher,
            source: entry.source,
            timestamp: entry.timestamp.and_utc().timestamp() as u64,
            publisher_signature: entry.publisher_signature,
            price: entry.price.to_u128().unwrap_or(0), // change default value ?
        }
    }
}
