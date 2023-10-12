use axum::extract::State;
use axum::Json;
use chrono::NaiveDateTime;
use starknet::core::crypto::{ecdsa_verify, Signature};
use starknet::core::types::FieldElement;

use crate::domain::models::entry::EntryError;
use crate::handlers::entries::{CreateEntryRequest, EntryResponse};
use crate::infra::repositories::entry_repository;
use crate::utils::JsonExtractor;
use crate::AppState;

pub async fn create_entry(
    State(state): State<AppState>,
    JsonExtractor(new_entry): JsonExtractor<CreateEntryRequest>,
) -> Result<Json<EntryResponse>, EntryError> {
    // TODO: Fetch public key from database
    let public_key = FieldElement::ZERO;
    // TODO: Compute message hash
    let message_hash = FieldElement::ZERO;

    if !ecdsa_verify(
        &public_key,
        &message_hash,
        &Signature {
            r: new_entry.signature[0],
            s: new_entry.signature[1],
        },
    )
    .map_err(EntryError::InvalidSignature)?
    {
        return Err(EntryError::Unauthorized(new_entry.publisher));
    }

    let new_entry_db = entry_repository::NewEntryDb {
        pair_id: new_entry.pair_id,
        publisher: new_entry.publisher,
        source: new_entry.source,
        timestamp: NaiveDateTime::from_timestamp_opt(new_entry.timestamp as i64, 0).unwrap(), // TODO: remove unwrap
        price: new_entry.price.into(),
    };

    let created_entry = entry_repository::insert(&state.pool, new_entry_db)
        .await
        .map_err(EntryError::InfraError)?;

    let entry_response = EntryResponse {
        pair_id: created_entry.pair_id,
        timestamp: created_entry.timestamp,
        num_sources_aggregated: 0, // TODO: add real value
        price: created_entry.price,
    };

    Ok(Json(entry_response))
}
