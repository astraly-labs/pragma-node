use axum::extract::State;
use axum::Json;
use chrono::{DateTime, Utc};
use pragma_entities::{EntryError, NewFutureEntry, PublisherError};
use starknet::core::crypto::{ecdsa_verify, Signature};
use starknet::core::types::FieldElement;

use super::{CreateFutureEntryRequest, CreateFutureEntryResponse};
use crate::types::entries::build_publish_message;

use crate::config::config;
use crate::infra::kafka;
use crate::infra::repositories::publisher_repository;
use crate::utils::JsonExtractor;
use crate::AppState;

#[utoipa::path(
    post,
    path = "/node/v1/data/publish_future",
    request_body = CreatePerpEntryRequest,
    responses(
        (status = 200, description = "Entries published successfuly", body = CreatePerpEntryResponse),
        (status = 401, description = "Unauthorized Publisher", body = EntryError)
    )
)]
pub async fn create_future_entries(
    State(state): State<AppState>,
    JsonExtractor(new_entries): JsonExtractor<CreateFutureEntryRequest>,
) -> Result<Json<CreateFutureEntryResponse>, EntryError> {
    tracing::info!("Received new future entries: {:?}", new_entries);

    let config = config().await;

    if new_entries.entries.is_empty() {
        return Ok(Json(CreateFutureEntryResponse {
            number_entries_created: 0,
        }));
    }

    let publisher_name = new_entries.entries[0].base.publisher.clone();

    let publisher = publisher_repository::get(&state.offchain_pool, publisher_name.clone())
        .await
        .map_err(EntryError::InfraError)?;

    // Check if publisher is active
    publisher.assert_is_active()?;

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
    let account_address = publisher_repository::get(&state.offchain_pool, publisher_name.clone())
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

    let published_message = match build_publish_message(&new_entries.entries, None) {
        Ok(message) => message,
        Err(_) => {
            // If the new version fails, try the legacy version
            match build_publish_message(&new_entries.entries, Some(true)) {
                Ok(message) => message,
                Err(err) => panic!("Failed to build publish message: {:?}", err),
            }
        }
    };
    let message_hash = published_message.message_hash(account_address);
    let signature = Signature {
        r: new_entries.signature[0],
        s: new_entries.signature[1],
    };

    if !ecdsa_verify(&public_key, &message_hash, &signature)
        .map_err(EntryError::InvalidSignature)?
    {
        tracing::error!("Invalid signature for message hash {:?}", &message_hash);
        return Err(EntryError::Unauthorized);
    }

    let new_entries_db = new_entries
        .entries
        .iter()
        .map(|future_entry| {
            let dt = match DateTime::<Utc>::from_timestamp(future_entry.base.timestamp as i64, 0) {
                Some(dt) => dt.naive_utc(),
                None => return Err(EntryError::InvalidTimestamp),
            };

            // For expiration_timestamp, 0 is sent by publishers for perpetual entries.
            // We set them to None in the database to easily filter them out.
            let expiry_dt = if future_entry.expiration_timestamp == 0 {
                None
            } else {
                match DateTime::<Utc>::from_timestamp_millis(
                    future_entry.expiration_timestamp as i64,
                ) {
                    Some(dt) => Some(dt.naive_utc()),
                    None => return Err(EntryError::InvalidTimestamp),
                }
            };

            Ok(NewFutureEntry {
                pair_id: future_entry.pair_id.clone(),
                publisher: future_entry.base.publisher.clone(),
                source: future_entry.base.source.clone(),
                timestamp: dt,
                expiration_timestamp: expiry_dt,
                publisher_signature: format!("0x{}", signature),
                price: future_entry.price.into(),
            })
        })
        .collect::<Result<Vec<NewFutureEntry>, EntryError>>()?;

    let data =
        serde_json::to_vec(&new_entries_db).map_err(|e| EntryError::PublishData(e.to_string()))?;

    if let Err(e) = kafka::send_message(config.kafka_topic(), &data, &publisher_name).await {
        tracing::error!("Error sending message to kafka: {:?}", e);
        return Err(EntryError::PublishData(String::from(
            "Error sending message to kafka",
        )));
    };

    Ok(Json(CreateFutureEntryResponse {
        number_entries_created: new_entries.entries.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    use crate::types::entries::{FutureEntry, PerpEntry};

    #[rstest]
    fn test_build_publish_message_empty() {
        let entries: Vec<PerpEntry> = vec![];
        let typed_data = build_publish_message(&entries, None).unwrap();
        assert_eq!(typed_data.primary_type, "Request");
        assert_eq!(typed_data.domain.name, "Pragma");
        assert_eq!(typed_data.domain.version, "1");
        assert_eq!(typed_data.message.action, "Publish");
        assert_eq!(typed_data.message.entries, entries);

        let entries: Vec<FutureEntry> = vec![];
        let typed_data = build_publish_message(&entries, None).unwrap();
        assert_eq!(typed_data.primary_type, "Request");
        assert_eq!(typed_data.domain.name, "Pragma");
        assert_eq!(typed_data.domain.version, "1");
        assert_eq!(typed_data.message.action, "Publish");
        assert_eq!(typed_data.message.entries, entries);
    }
}
