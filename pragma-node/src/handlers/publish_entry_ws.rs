use std::net::SocketAddr;
use std::sync::Arc;

use crate::handlers::create_entry::{CreateEntryRequest, CreateEntryResponse};
use crate::types::auth::{build_login_message, LoginMessage};
use crate::types::entries::Entry;
use crate::types::ws::{ChannelHandler, Subscriber, WebSocketError};
use crate::utils::{
    assert_login_is_valid, assert_request_signature_is_valid, convert_entry_to_db,
    publish_to_kafka, validate_publisher,
};
use crate::AppState;

use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};

use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use starknet_crypto::Signature;

#[derive(Debug, Deserialize)]
enum ClientMessage {
    Login(LoginMessage),
    Publish(CreateEntryRequest),
}

#[derive(Debug, Default)]
pub struct PublishEntryState;

#[tracing::instrument(skip(state, ws), fields(endpoint_name = "publish_entry"))]
pub async fn publish_entry(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    if state.pragma_signer.is_none() {
        return (StatusCode::LOCKED, "Locked: Pragma signer not found").into_response();
    }

    ws.on_upgrade(move |socket| create_new_subscriber(socket, state, client_addr))
}

/// Interval in milliseconds that the channel will update the client with the latest prices.
const CHANNEL_UPDATE_INTERVAL_IN_MS: u64 = 500;

#[tracing::instrument(
    skip(socket, app_state),
    fields(
        subscriber_id,
        client_ip = %client_addr.ip()
    )
)]
async fn create_new_subscriber(socket: WebSocket, app_state: AppState, client_addr: SocketAddr) {
    let (mut subscriber, _) = match Subscriber::<PublishEntryState>::new(
        "publish_entry".into(),
        socket,
        client_addr.ip(),
        Arc::new(app_state),
        None,
        CHANNEL_UPDATE_INTERVAL_IN_MS,
    )
    .await
    {
        Ok(subscriber) => subscriber,
        Err(e) => {
            tracing::error!("Failed to register subscriber: {}", e);
            return;
        }
    };

    // Main event loop for the subscriber
    let handler = PublishEntryHandler;
    let status = subscriber.listen(handler).await;
    if let Err(e) = status {
        tracing::error!(
            "[{}] Error occurred while listening to the subscriber: {:?}",
            subscriber.id,
            e
        );
    }
}

pub struct PublishEntryHandler;

#[derive(Debug, Serialize)]
struct PublishResponse {
    status: String,
    message: String,
    data: Option<CreateEntryResponse>,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    status: String,
    message: String,
}

impl ChannelHandler<PublishEntryState, ClientMessage, WebSocketError> for PublishEntryHandler {
    async fn handle_client_msg(
        &mut self,
        subscriber: &mut Subscriber<PublishEntryState>,
        client_message: ClientMessage,
    ) -> Result<(), WebSocketError> {
        match client_message {
            ClientMessage::Login(login_message) => {
                let result = process_login(subscriber, login_message).await;
                let has_login_failed = result.is_err();
                let response = match result {
                    Ok(_) => LoginResponse {
                        status: "success".to_string(),
                        message: "Login successful".to_string(),
                    },
                    Err(e) => LoginResponse {
                        status: "error".to_string(),
                        message: e.to_string(),
                    },
                };
                subscriber
                    .send_msg(serde_json::to_string(&response).unwrap())
                    .await
                    .map_err(|_| WebSocketError::ChannelClose)?;

                // If login was unsucessful we just close the channel
                if has_login_failed {
                    return Err(WebSocketError::ChannelClose);
                }
            }
            ClientMessage::Publish(new_entries) => {
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
            }
        }

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

#[tracing::instrument(skip(subscriber))]
async fn process_login(
    subscriber: &Subscriber<PublishEntryState>,
    login_message: LoginMessage,
) -> Result<(), EntryError> {
    let publisher_name = login_message.publisher_name;
    let state = subscriber.app_state.clone();

    let message = build_login_message(&publisher_name);

    let signature = &Signature {
        r: login_message.signature[0],
        s: login_message.signature[1],
    };

    match message {
        Ok(message) => {
            let publishers_cache = state.caches.publishers();
            let (public_key, account_address) =
                validate_publisher(&state.offchain_pool, &publisher_name, publishers_cache).await?;

            assert_login_is_valid(message, signature, &account_address, &public_key)?;
        }
        Err(e) => {
            tracing::error!("Failed to build login message: {}", e);
            return Err(EntryError::InvalidLoginMessage(e.to_string()));
        }
    }
    Ok(())
}
