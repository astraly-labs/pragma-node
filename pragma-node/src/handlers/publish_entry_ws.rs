use crate::handlers::create_entry::{CreateEntryRequest, CreateEntryResponse};
use crate::types::entries::Entry;
use crate::types::ws::{ChannelHandler, Subscriber, WebSocketError};
use crate::utils::{
    assert_request_signature_is_valid, convert_entry_to_db, publish_to_kafka, validate_publisher,
};
use pragma_entities::EntryError;
use serde::Serialize;

// State for the publish entries WebSocket channel
#[derive(Debug, Default)]
pub struct PublishEntryState {}

// Handler for the publish entries WebSocket channel
pub struct PublishEntryHandler {}

impl PublishEntryHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Serialize)]
struct PublishResponse {
    status: String,
    message: String,
    data: Option<CreateEntryResponse>,
}

impl ChannelHandler<PublishEntryState, CreateEntryRequest, WebSocketError> for PublishEntryHandler {
    async fn handle_client_msg(
        &mut self,
        subscriber: &mut Subscriber<PublishEntryState>,
        new_entries: CreateEntryRequest,
    ) -> Result<(), WebSocketError> {
        let result = process_entries(subscriber, new_entries).await;

        let response = match result {
            Ok(response) => PublishResponse {
                status: "success".to_string(),
                message: "Entries published successfully".to_string(),
                data: Some(response),
            },
            Err(e) => PublishResponse {
                status: "error".to_string(),
                message: e.to_string(),
                data: None,
            },
        };

        subscriber
            .send_msg(serde_json::to_string(&response).unwrap())
            .await
            .map_err(|_| WebSocketError::ChannelClose)?;

        Ok(())
    }

    async fn periodic_interval(
        &mut self,
        _subscriber: &mut Subscriber<PublishEntryState>,
    ) -> Result<(), WebSocketError> {
        // No periodic updates needed for this endpoint
        Ok(())
    }
}

#[tracing::instrument(skip(subscriber))]
async fn process_entries(
    subscriber: &Subscriber<PublishEntryState>,
    new_entries: CreateEntryRequest,
) -> Result<CreateEntryResponse, EntryError> {
    tracing::info!("Received new entries via WebSocket: {:?}", new_entries);

    if new_entries.entries.is_empty() {
        return Ok(CreateEntryResponse {
            number_entries_created: 0,
        });
    }

    let publisher_name = new_entries.entries[0].base.publisher.clone();
    let publishers_cache = subscriber.app_state.caches.publishers();
    let (public_key, account_address) = validate_publisher(
        &subscriber.app_state.offchain_pool,
        &publisher_name,
        publishers_cache,
    )
    .await?;

    let signature = assert_request_signature_is_valid::<CreateEntryRequest, Entry>(
        &new_entries,
        &account_address,
        &public_key,
    )?;

    let new_entries_db = new_entries
        .entries
        .iter()
        .map(|entry| convert_entry_to_db(entry, &signature))
        .collect::<Result<Vec<_>, EntryError>>()?;

    let config = crate::config::config().await;
    publish_to_kafka(
        new_entries_db,
        config.kafka_topic().to_string(),
        &publisher_name,
    )
    .await?;

    Ok(CreateEntryResponse {
        number_entries_created: new_entries.entries.len(),
    })
}
