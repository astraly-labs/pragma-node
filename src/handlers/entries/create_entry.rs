use axum::extract::State;
use axum::Json;
use chrono::NaiveDateTime;
use starknet::core::crypto::{ecdsa_verify, Signature};
use starknet::core::types::FieldElement;

use crate::domain::models::entry::EntryError;
use crate::domain::models::publisher::PublisherError;
use crate::handlers::entries::{CreateEntryRequest, CreateEntryResponse};
use crate::infra::repositories::{entry_repository, publisher_repository};
use crate::utils::JsonExtractor;
use crate::AppState;

pub async fn create_entries(
    State(state): State<AppState>,
    JsonExtractor(new_entries): JsonExtractor<CreateEntryRequest>,
) -> Result<Json<CreateEntryResponse>, EntryError> {
    tracing::info!("Received new entries: {:?}", new_entries);

    if new_entries.entries.is_empty() {
        return Ok(Json(CreateEntryResponse {
            number_entries_created: 0,
        }));
    }

    let publisher_name = new_entries.entries[0].base.publisher.clone();

    // Fetch public key from database
    let public_key = publisher_repository::get(&state.pool, publisher_name.clone())
        .await
        .map_err(EntryError::InfraError)?
        .active_key;
    let public_key = FieldElement::from_hex_be(&public_key)
        .map_err(|_| EntryError::PublisherError(PublisherError::InvalidKey(public_key)))?;

    tracing::info!(
        "Retrieved {:?} public key: {:?}",
        publisher_name,
        public_key
    );

    // TODO: Compute message hash
    let message_hash = FieldElement::ZERO;

    if !ecdsa_verify(
        &public_key,
        &message_hash,
        &Signature {
            r: new_entries.signature[0],
            s: new_entries.signature[1],
        },
    )
    .map_err(EntryError::InvalidSignature)?
    {
        return Err(EntryError::Unauthorized);
    }

    // Iterate over new entries
    for new_entry in &new_entries.entries {
        let new_entry_db = entry_repository::NewEntryDb {
            pair_id: new_entry.pair_id.clone(),
            publisher: new_entry.base.publisher.clone(),
            source: new_entry.base.source.clone(),
            timestamp: NaiveDateTime::from_timestamp_opt(new_entry.base.timestamp as i64, 0)
                .unwrap(), // TODO: remove unwrap
            price: new_entry.price.into(),
        };

        let _created_entry = entry_repository::insert(&state.pool, new_entry_db)
            .await
            .map_err(EntryError::InfraError)?;
    }

    Ok(Json(CreateEntryResponse {
        number_entries_created: new_entries.entries.len(),
    }))
}
