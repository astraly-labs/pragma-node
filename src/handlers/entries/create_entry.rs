use axum::extract::State;
use axum::Json;
use axum_macros::debug_handler;
use chrono::NaiveDateTime;

use crate::domain::models::entry::EntryError;
use crate::handlers::entries::{CreateEntryRequest, EntryResponse};
use crate::infra::repositories::entry_repository;
use crate::utils::JsonExtractor;
use crate::AppState;

#[debug_handler]
pub async fn create_entry(
    State(state): State<AppState>,
    JsonExtractor(new_entry): JsonExtractor<CreateEntryRequest>,
) -> Result<Json<EntryResponse>, EntryError> {
    // TODO: Verify Signature

    let new_entry_db = entry_repository::NewEntryDb {
        pair_id: new_entry.pair_id,
        publisher: new_entry.publisher,
        timestamp: NaiveDateTime::from_timestamp_opt(new_entry.timestamp as i64, 0).unwrap(), // TODO: remove unwrap
        price: new_entry.price.into(),
    };

    let created_entry = entry_repository::insert(&state.pool, new_entry_db)
        .await
        .map_err(EntryError::InfraError)?;

    let entry_response = EntryResponse {
        pair_id: created_entry.pair_id,
        timestamp: created_entry.timestamp,
        num_sources_aggregated: 0,
        price: created_entry.price,
    };

    Ok(Json(entry_response))
}
