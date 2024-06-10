use crate::config::config;
use crate::infra::kafka;
use crate::infra::repositories::publisher_repository;
use crate::utils::{JsonExtractor, TypedData};
use crate::AppState;
use axum::extract::State;
use axum::Json;
use chrono::{DateTime, Utc};
use pragma_entities::{EntryError, NewEntry, PublisherError};
use serde::{Deserialize, Serialize};
use starknet::core::crypto::{ecdsa_verify, Signature};
use starknet::core::types::FieldElement;

use super::{CreatePerpEntryRequest, CreatePerpEntryResponse, PerpEntry};

#[derive(Debug, Serialize, Deserialize)]
struct PublishMessage {
    action: String,
    perp_entries: Vec<PerpEntry>,
}

// TODO(akhercha): double-check this
fn build_publish_message(
    perp_entries: &[PerpEntry],
) -> Result<TypedData<PublishMessage>, EntryError> {
    // Construct the raw string with placeholders for the entries
    let raw_message = format!(
        r#"{{
            "domain": {{"name": "Pragma", "version": "1"}},
            "primaryType": "Request",
            "message": {{
                "action": "Publish",
                "perp_entries": {}
            }},
            "types": {{
                "StarkNetDomain": [
                    {{"name": "name", "type": "felt"}},
                    {{"name": "version", "type": "felt"}}
                ],
                "Request": [
                    {{"name": "action", "type": "felt"}},
                    {{"name": "perp_entries", "type": "PerpPentry*"}}
                ],
                "PerpEntry": [
                    {{"name": "base", "type": "Base"}},
                    {{"name": "pair_id", "type": "felt"}},
                    {{"name": "price", "type": "felt"}},
                    {{"name": "volume", "type": "felt"}}
                ],
                "Base": [
                    {{"name": "publisher", "type": "felt"}},
                    {{"name": "source", "type": "felt"}},
                    {{"name": "timestamp", "type": "felt"}}
                ]
            }}
        }}"#,
        serde_json::to_string(perp_entries).map_err(|e| EntryError::BuildPublish(e.to_string()))?
    );

    serde_json::from_str(&raw_message).map_err(|e| EntryError::BuildPublish(e.to_string()))
}

#[utoipa::path(
    post,
    path = "/node/v1/data/publish/perp",
    request_body = CreatePerpEntryRequest,
    responses(
        (status = 200, description = "Entries published successfuly", body = CreatePerpEntryResponse),
        (status = 401, description = "Unauthorized Publisher", body = EntryError)
    )
)]
pub async fn create_perp_entries(
    State(state): State<AppState>,
    JsonExtractor(new_perp_entries): JsonExtractor<CreatePerpEntryRequest>,
) -> Result<Json<CreatePerpEntryResponse>, EntryError> {
    tracing::info!("Received new perp entries: {:?}", new_perp_entries);

    let config = config().await;

    if new_perp_entries.perp_entries.is_empty() {
        return Ok(Json(CreatePerpEntryResponse {
            number_entries_created: 0,
        }));
    }

    let publisher_name = new_perp_entries.perp_entries[0].base.publisher.clone();

    let publisher = publisher_repository::get(&state.timescale_pool, publisher_name.clone())
        .await
        .map_err(EntryError::InfraError)?;

    // Check if publisher is active
    if !publisher.active {
        tracing::error!("Publisher {:?} is not active", publisher_name);
        return Err(EntryError::PublisherError(
            PublisherError::InactivePublisher(publisher_name),
        ));
    }

    // Fetch public key from database
    // TODO: Fetch it from contract
    let public_key = publisher.active_key;
    let public_key = FieldElement::from_hex_be(&public_key)
        .map_err(|_| EntryError::PublisherError(PublisherError::InvalidKey(public_key)))?;

    tracing::info!(
        "Retrieved {:?} public key: {:?}",
        publisher_name,
        public_key
    );

    // Fetch account address from database
    // TODO: Cache it
    let account_address = publisher_repository::get(&state.timescale_pool, publisher_name.clone())
        .await
        .map_err(EntryError::InfraError)?
        .account_address;
    let account_address = FieldElement::from_hex_be(&account_address)
        .map_err(|_| EntryError::PublisherError(PublisherError::InvalidAddress(account_address)))?;

    tracing::info!(
        "Retrieved {:?} account address: {:?}",
        publisher_name,
        account_address
    );

    let message_hash =
        build_publish_message(&new_perp_entries.perp_entries)?.message_hash(account_address);
    let signature = Signature {
        r: new_perp_entries.signature[0],
        s: new_perp_entries.signature[1],
    };

    if !ecdsa_verify(&public_key, &message_hash, &signature)
        .map_err(EntryError::InvalidSignature)?
    {
        tracing::error!("Invalid signature for message hash {:?}", &message_hash);
        return Err(EntryError::Unauthorized);
    }

    let new_perp_entries_db = new_perp_entries
        .perp_entries
        .iter()
        .map(|perp_entry| {
            let dt = match DateTime::<Utc>::from_timestamp(perp_entry.base.timestamp as i64, 0) {
                Some(dt) => dt.naive_utc(),
                None => return Err(EntryError::InvalidTimestamp),
            };

            Ok(NewEntry {
                pair_id: perp_entry.pair_id.clone(),
                publisher: perp_entry.base.publisher.clone(),
                source: perp_entry.base.source.clone(),
                timestamp: dt,
                publisher_signature: format!("0x{}", signature),
                price: perp_entry.price.into(),
            })
        })
        .collect::<Result<Vec<NewEntry>, EntryError>>()?;

    let data = serde_json::to_vec(&new_perp_entries_db)
        .map_err(|e| EntryError::PublishData(e.to_string()))?;

    if let Err(e) = kafka::send_message(config.kafka_topic(), &data, &publisher_name).await {
        tracing::error!("Error sending message to kafka: {:?}", e);
        return Err(EntryError::PublishData(String::from(
            "Error sending message to kafka",
        )));
    };

    Ok(Json(CreatePerpEntryResponse {
        number_entries_created: new_perp_entries.perp_entries.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_build_publish_message_empty() {
        let perp_entries = vec![];
        let typed_data = build_publish_message(&perp_entries).unwrap();

        assert_eq!(typed_data.primary_type, "Request");
        assert_eq!(typed_data.domain.name, "Pragma");
        assert_eq!(typed_data.domain.version, "1");
        assert_eq!(typed_data.message.action, "Publish");
        assert_eq!(typed_data.message.perp_entries, perp_entries);
    }
}
