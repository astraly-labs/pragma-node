use crate::config::config;
use crate::infra::kafka;
use crate::infra::repositories::publisher_repository;
use crate::utils::{JsonExtractor, TypedData};
use crate::AppState;
use crate::requests::{CreatePerpRequest, SpotTimeStamp, CreatePerpResponse, Entry};
use axum::extract::State;
use axum::Json;
use chrono::NaiveDateTime;
use pragma_entities::{EntryError, NewEntry, PublisherError};
use serde::{Deserialize, Serialize};
use starknet::core::crypto::{ecdsa_verify, Signature};
use starknet::core::types::FieldElement;

#[derive(Debug, Serialize, Deserialize)]
pub struct PublishMessage {
    action: String,
    entries: Vec<Entry>,
}

pub(crate) fn build_publish_message(
    entries: &Vec<Entry>,
) -> Result<TypedData<PublishMessage>, EntryError> {
    // Construct the raw string with placeholders for the entries
    let raw_message = format!(
        r#"{{
            "domain": {{"name": "Pragma", "version": "1"}},
            "primaryType": "Request",
            "message": {{
                "action": "Publish",
                "entries": {}
            }},
            "types": {{
                "StarkNetDomain": [
                    {{"name": "name", "type": "felt"}},
                    {{"name": "version", "type": "felt"}}
                ],
                "Request": [
                    {{"name": "action", "type": "felt"}},
                    {{"name": "entries", "type": "Entry*"}}
                ],
                "Entry": [
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
        serde_json::to_string(entries).map_err(|e| EntryError::BuildPublish(e.to_string()))?
    );

    serde_json::from_str(&raw_message).map_err(|e| EntryError::BuildPublish(e.to_string()))
}

#[utoipa::path(
        post,
        path = "/node/v1/data/spot",
        request_body = CreatePerpRequest,
        responses(
            (status = 200, description = "Entries published successfuly", body = CreateSpotResponse),
            (status = 401, description = "Unauthorized Publisher", body = EntryError)
        )
    )]
pub async fn create_perps(
    State(state): State<AppState>,
    JsonExtractor(new_perps): JsonExtractor<CreatePerpRequest>,
) -> Result<Json<CreatePerpResponse>, EntryError> {
    tracing::info!("Received new perps: {:?}", new_perps);

    let config = config().await;

    if new_perps.entries.is_empty() {
        return Ok(Json(CreatePerpResponse {
            data_range: SpotTimeStamp {
                start_timestamp: new_perps.data_range.start_timestamp,
                end_timestamp: new_perps.data_range.end_timestamp,
            },
            .. Default::default()
        }));
    }

    let publisher_name = new_perps.entries[0].base.publisher.clone();

    let publisher = publisher_repository::get(&state.pool, publisher_name.clone())
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
    let account_address = publisher_repository::get(&state.pool, publisher_name.clone())
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

    let message_hash = build_publish_message(&new_perps.entries)?.message_hash(account_address);

    if !ecdsa_verify(
        &public_key,
        &message_hash,
        &Signature {
            r: new_perps.signature[0],
            s: new_perps.signature[1],
        },
    )
    .map_err(EntryError::InvalidSignature)?
    {
        tracing::error!("Invalid signature for message hash {:?}", &message_hash);
        return Err(EntryError::Unauthorized);
    }

    let new_entries_db = new_perps
        .entries
        .iter()
        .map(|entry| NewEntry {
            pair_id: entry.pair_id.clone(),
            publisher: entry.base.publisher.clone(),
            source: entry.base.source.clone(),
            timestamp: NaiveDateTime::from_timestamp_opt(entry.base.timestamp as i64, 0).unwrap(), // TODO: remove unwrap
            price: entry.price.into(),
        })
        .collect::<Vec<NewEntry>>();

    let data =
        serde_json::to_vec(&new_entries_db).map_err(|e| EntryError::PublishData(e.to_string()))?;

    if let Err(e) = kafka::send_message(config.kafka_topic(), &data, &publisher_name).await {
        tracing::error!("Error sending message to kafka: {:?}", e);
        return Err(EntryError::PublishData(String::from(
            "Error sending message to kafka",
        )));
    };

    Ok(Json(CreatePerpResponse {
        pair_id: new_perps.pair_id,
        price: new_perps.price,
        data_range: SpotTimeStamp {
            start_timestamp: new_perps.data_range.start_timestamp,
            end_timestamp: new_perps.data_range.end_timestamp,
        },
        volume: new_perps.volume,
        .. Default::default()
    }))
}

#[cfg(test)]
mod tests {
    use crate::requests::BaseEntry;

    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_build_publish_message_empty() {
        let entries = vec![];
        let typed_data = build_publish_message(&entries).unwrap();

        assert_eq!(typed_data.primary_type, "Request");
        assert_eq!(typed_data.domain.name, "Pragma");
        assert_eq!(typed_data.domain.version, "1");
        assert_eq!(typed_data.message.action, "Publish");
        assert_eq!(typed_data.message.entries, entries);
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
        assert_eq!(typed_data.message.action, "Publish");
        assert_eq!(typed_data.message.entries, entries);

        let msg_hash = typed_data.message_hash(FieldElement::ZERO);
        // Hash computed with the Pragma SDK (python)
        assert_eq!(msg_hash, FieldElement::from_hex_be("").unwrap());
    }
}
