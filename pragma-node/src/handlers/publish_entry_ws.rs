use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use utoipa::ToSchema;

use crate::handlers::create_entry::CreateEntryResponse;
use crate::types::auth::{build_login_message, LoginMessage};
use crate::types::entries::Entry;
use crate::types::ws::{ChannelHandler, Subscriber, WebSocketError};
use crate::utils::{
    assert_login_is_valid, convert_entry_to_db, publish_to_kafka, validate_publisher,
};
use crate::AppState;

use pragma_entities::EntryError;
use starknet_crypto::{Felt, Signature};

use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

// Session expiry time in minutes
const SESSION_EXPIRY_MINUTES: u64 = 5;

#[derive(Debug)]
pub struct PublisherSession {
    login_time: SystemTime,
}

impl PublisherSession {
    fn new() -> Self {
        Self {
            login_time: SystemTime::now(),
        }
    }

    fn is_expired(&self) -> bool {
        SystemTime::now()
            .duration_since(self.login_time)
            .map(|duration| duration > Duration::from_secs(SESSION_EXPIRY_MINUTES * 60))
            .unwrap_or(true)
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PublishEntryRequest {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "msg_type")]
enum ClientMessage {
    #[serde(rename = "publish")]
    Publish(PublishEntryRequest),
    #[serde(rename = "login")]
    Login(LoginMessage),
}

#[derive(Debug, Default)]
pub struct PublishEntryState {
    publisher_name: Option<String>,
    is_logged_in: bool,
}

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
        Some(PublishEntryState {
            publisher_name: None,
            is_logged_in: false,
        }),
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

    // Clean up session on disconnect
    let state = subscriber.state.lock().await;
    if let Some(publisher_name) = &state.publisher_name {
        subscriber
            .app_state
            .publisher_sessions
            .remove(publisher_name);
    }

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
        let app_state = subscriber.app_state.clone();
        match client_message {
            ClientMessage::Login(login_message) => {
                // Check if this publisher already has an active session
                if let Some(session) = app_state
                    .publisher_sessions
                    .get(&login_message.publisher_name)
                {
                    if !session.is_expired() {
                        let response = LoginResponse {
                            status: "error".to_string(),
                            message: "Publisher already has an active session".to_string(),
                        };
                        subscriber
                            .send_msg(serde_json::to_string(&response).unwrap())
                            .await
                            .map_err(|_| WebSocketError::ChannelClose)?;
                        return Err(WebSocketError::ChannelClose);
                    }
                    // Remove expired session
                    subscriber
                        .app_state
                        .publisher_sessions
                        .remove(&login_message.publisher_name);
                }

                let result = process_login(subscriber, login_message.clone()).await;
                let has_login_failed = result.is_err();
                let response = match result {
                    Ok(_) => {
                        // Store the new session
                        subscriber.app_state.publisher_sessions.insert(
                            login_message.publisher_name.clone(),
                            PublisherSession::new(),
                        );
                        // Update subscriber state
                        {
                            let mut state = subscriber.state.lock().await;
                            *state = PublishEntryState {
                                publisher_name: Some(login_message.publisher_name),
                                is_logged_in: true,
                            };
                        }
                        LoginResponse {
                            status: "success".to_string(),
                            message: "Login successful".to_string(),
                        }
                    }
                    Err(e) => LoginResponse {
                        status: "error".to_string(),
                        message: e.to_string(),
                    },
                };
                subscriber
                    .send_msg(serde_json::to_string(&response).unwrap())
                    .await
                    .map_err(|_| WebSocketError::ChannelClose)?;

                // If login was unsuccessful we just close the channel
                if has_login_failed {
                    return Err(WebSocketError::ChannelClose);
                }
            }
            ClientMessage::Publish(new_entries) => {
                // Check login state and session expiry
                let should_send_error = {
                    let state = subscriber.state.lock().await;

                    if !state.is_logged_in {
                        Some(PublishResponse {
                            status: "error".to_string(),
                            message: "Not logged in".to_string(),
                            data: None,
                        })
                    } else if let Some(publisher_name) = &state.publisher_name {
                        if let Some(session) =
                            subscriber.app_state.publisher_sessions.get(publisher_name)
                        {
                            if session.is_expired() {
                                subscriber
                                    .app_state
                                    .publisher_sessions
                                    .remove(publisher_name);
                                Some(PublishResponse {
                                    status: "error".to_string(),
                                    message: "Session expired, please login again".to_string(),
                                    data: None,
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                if let Some(error_response) = should_send_error {
                    subscriber
                        .send_msg(serde_json::to_string(&error_response).unwrap())
                        .await
                        .map_err(|_| WebSocketError::ChannelClose)?;
                    if error_response.message.contains("expired") {
                        return Err(WebSocketError::ChannelClose);
                    }
                    return Ok(());
                }

                // Process entries without signature verification
                let result = process_entries_without_verification(subscriber, new_entries).await;
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
        subscriber: &mut Subscriber<PublishEntryState>,
    ) -> Result<(), WebSocketError> {
        // Check session expiry periodically
        let should_close = {
            let state = subscriber.state.lock().await;
            if let Some(publisher_name) = &state.publisher_name {
                if let Some(session) = subscriber.app_state.publisher_sessions.get(publisher_name) {
                    if session.is_expired() {
                        subscriber
                            .app_state
                            .publisher_sessions
                            .remove(publisher_name);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };

        if should_close {
            let response = PublishResponse {
                status: "error".to_string(),
                message: "Session expired, please login again".to_string(),
                data: None,
            };
            subscriber
                .send_msg(serde_json::to_string(&response).unwrap())
                .await
                .map_err(|_| WebSocketError::ChannelClose)?;
            return Err(WebSocketError::ChannelClose);
        }
        Ok(())
    }
}

#[tracing::instrument(skip(subscriber))]
async fn process_entries_without_verification(
    subscriber: &Subscriber<PublishEntryState>,
    new_entries: PublishEntryRequest,
) -> Result<CreateEntryResponse, EntryError> {
    tracing::info!("Received new entries via WebSocket: {:?}", new_entries);

    if new_entries.entries.is_empty() {
        return Ok(CreateEntryResponse {
            number_entries_created: 0,
        });
    }

    let state = subscriber.state.lock().await;
    let publisher_name = state
        .publisher_name
        .as_ref()
        .ok_or_else(|| EntryError::NotFound("No publisher name in session state".to_string()))?;

    let new_entries_db = new_entries
        .entries
        .iter()
        .map(|entry| {
            convert_entry_to_db(
                entry,
                &Signature {
                    r: Felt::ZERO,
                    s: Felt::ZERO,
                },
            )
        })
        .collect::<Result<Vec<_>, EntryError>>()?;

    let config = crate::config::config().await;
    publish_to_kafka(
        new_entries_db,
        config.kafka_topic().to_string(),
        publisher_name,
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

    // Check if the expiration timestamp is valid
    let current_time = chrono::Utc::now().timestamp() as u64;
    if login_message.expiration_timestamp <= current_time {
        return Err(EntryError::InvalidLoginMessage(
            "Login message has expired".to_string(),
        ));
    }

    let message = build_login_message(&publisher_name, login_message.expiration_timestamp);

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
