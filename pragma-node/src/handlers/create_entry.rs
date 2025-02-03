use axum::extract::{self, State};
use axum::Json;
use pragma_entities::{EntryError, NewEntry};
use serde::{Deserialize, Serialize};
use starknet::core::types::Felt;
use utoipa::{ToResponse, ToSchema};

use crate::config::config;
use crate::types::entries::Entry;
use crate::utils::{
    assert_request_signature_is_valid, convert_entry_to_db, felt_from_decimal, publish_to_kafka,
    validate_publisher,
};
use crate::AppState;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateEntryRequest {
    #[schema(value_type = Vec<String>)]
    #[serde(deserialize_with = "felt_from_decimal")]
    pub signature: Vec<Felt>,
    pub entries: Vec<Entry>,
}

impl AsRef<[Felt]> for CreateEntryRequest {
    fn as_ref(&self) -> &[Felt] {
        &self.signature
    }
}

impl AsRef<[Entry]> for CreateEntryRequest {
    fn as_ref(&self) -> &[Entry] {
        &self.entries
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, ToResponse)]
pub struct CreateEntryResponse {
    pub number_entries_created: usize,
}

#[utoipa::path(
    post,
    path = "/node/v1/data/publish",
    request_body = CreateEntryRequest,
    responses(
        (status = 200, description = "Entries published successfuly", body = CreateEntryResponse),
        (status = 401, description = "Unauthorized Publisher", body = EntryError)
    )
)]
#[tracing::instrument(skip(state))]
pub async fn create_entries(
    State(state): State<AppState>,
    extract::Json(new_entries): extract::Json<CreateEntryRequest>,
) -> Result<Json<CreateEntryResponse>, EntryError> {
    tracing::info!("Received new entries: {:?}", new_entries);

    if new_entries.entries.is_empty() {
        return Ok(Json(CreateEntryResponse {
            number_entries_created: 0,
        }));
    }

    let publisher_name = new_entries.entries[0].base.publisher.clone();
    let publishers_cache = state.caches.publishers();
    let (public_key, account_address) =
        validate_publisher(&state.offchain_pool, &publisher_name, publishers_cache).await?;

    let signature = assert_request_signature_is_valid::<CreateEntryRequest, Entry>(
        &new_entries,
        &account_address,
        &public_key,
    )?;

    let new_entries_db = new_entries
        .entries
        .iter()
        .map(|entry| convert_entry_to_db(entry, &signature))
        .collect::<Result<Vec<NewEntry>, EntryError>>()?;

    let config = config().await;
    publish_to_kafka(
        new_entries_db,
        config.kafka_topic().to_string(),
        &publisher_name,
    )
    .await?;

    Ok(Json(CreateEntryResponse {
        number_entries_created: new_entries.entries.len(),
    }))
}

#[cfg(test)]
mod tests {
    use crate::types::entries::{build_publish_message, BaseEntry, Entry};

    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_build_publish_message_empty() {
        let entries: Vec<Entry> = vec![];
        let typed_data = build_publish_message(&entries).unwrap();

        assert_eq!(typed_data.primary_type, "Request");
        assert_eq!(typed_data.domain.name, "Pragma");
        assert_eq!(typed_data.domain.version, "1");
        // assert_eq!(typed_data.message.action, "Publish");
        // assert_eq!(typed_data.message.entries, entries);
    }

    #[rstest]
    #[ignore = "TODO: Compute hash with Pragma SDK"]
    fn test_build_publish_message() {
        let entries = vec![Entry {
            base: BaseEntry {
                timestamp: 0,
                source: "source".to_string(),
                publisher: "publisher".to_string(),
            },
            pair_id: "pair_id".to_string(),
            price: 0,
            volume: 0,
        }];
        let typed_data = build_publish_message(&entries).unwrap();

        assert_eq!(typed_data.primary_type, "Request");
        assert_eq!(typed_data.domain.name, "Pragma");
        assert_eq!(typed_data.domain.version, "1");
        // assert_eq!(typed_data.message.action, "Publish");
        // assert_eq!(typed_data.message.entries, entries);

        let msg_hash = typed_data.encode(Felt::ZERO).unwrap().message_hash;
        // Hash computed with the Pragma SDK (python)
        assert_eq!(msg_hash, Felt::from_hex("").unwrap());
    }
}
