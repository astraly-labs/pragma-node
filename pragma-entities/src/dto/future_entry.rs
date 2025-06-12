use bigdecimal::ToPrimitive;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, ToSchema)]
pub struct FutureEntry {
    pub id: Uuid,
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: u64,
    pub expiration_timestamp: Option<u64>,
    pub publisher_signature: String,
    pub price: u128,
}

impl From<crate::FutureEntry> for FutureEntry {
    fn from(future_entry: crate::FutureEntry) -> Self {
        let expiration_timestamp = future_entry
            .expiration_timestamp
            .map(|expiration_timestamp| expiration_timestamp.and_utc().timestamp_millis() as u64);

        Self {
            id: future_entry.id,
            pair_id: future_entry.pair_id,
            publisher: future_entry.publisher,
            source: future_entry.source,
            timestamp: future_entry.timestamp.and_utc().timestamp_millis() as u64,
            expiration_timestamp,
            publisher_signature: future_entry.publisher_signature.unwrap_or_default(),
            price: future_entry.price.to_u128().unwrap_or(0), // change default value ?
        }
    }
}
