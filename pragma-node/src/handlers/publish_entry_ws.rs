use pragma_entities::error::WebSocketError;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use utoipa::ToSchema;

use pragma_common::types::auth::{LoginMessage, build_login_message};
use pragma_common::types::entries::MarketEntry;
use pragma_entities::EntryError;
use pragma_entities::{NewEntry, NewFutureEntry};
use starknet_crypto::{Felt, Signature};

use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;

use crate::handlers::create_entry::CreateEntryResponse;
use crate::state::AppState;
use crate::utils::{ChannelHandler, Subscriber, convert_entry_to_db, convert_perp_entry_to_db};
use crate::utils::{publish_to_kafka, validate_publisher};
use pragma_common::signing::assert_login_is_valid;

// Session expiry time in minutes
const SESSION_EXPIRY_DURATION: Duration = Duration::from_secs(5 * 60);

#[derive(Debug)]
pub struct PublisherSession {
    login_time: SystemTime,
    ip_address: std::net::IpAddr,
}

impl PublisherSession {
    fn new(ip_address: std::net::IpAddr) -> Self {
        Self {
            login_time: SystemTime::now(),
            ip_address,
        }
    }

    /// Checks if the session has expired
    /// In that case the publisher should login again
    fn is_expired(&self) -> bool {
        SystemTime::now()
            .duration_since(self.login_time)
            .map(|duration| duration > SESSION_EXPIRY_DURATION)
            .unwrap_or(true)
    }

    /// Checks if the IP address matches the one stored in the session
    /// This is used to check if the publisher is sending entries from the same IP address he logged in from
    fn validate_ip(&self, ip: &std::net::IpAddr) -> bool {
        &self.ip_address == ip
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PublishEntryRequest {
    pub entries: Vec<MarketEntry>,
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
    let mut subscriber = match Subscriber::<PublishEntryState>::new(
        "publish_entry".into(),
        socket,
        client_addr.ip(),
        Arc::new(app_state),
        Some(PublishEntryState {
            publisher_name: None,
            is_logged_in: false,
        }),
        CHANNEL_UPDATE_INTERVAL_IN_MS,
        None,
    ) {
        Ok(subscriber) => subscriber,
        Err(e) => {
            tracing::error!("Failed to register subscriber: {:?}", e);
            return;
        }
    };

    // Main event loop for the subscriber
    let handler = PublishEntryHandler;
    let status = subscriber.listen(handler).await;

    // Clean up session on disconnect
    let publisher_name = &subscriber.state.read().await.publisher_name;
    if let Some(publisher_name) = publisher_name {
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

#[derive(Debug, Serialize, Clone)]
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

#[async_trait::async_trait]
impl ChannelHandler<PublishEntryState, ClientMessage, WebSocketError> for PublishEntryHandler {
    #[allow(clippy::too_many_lines)]
    async fn handle_client_msg(
        &mut self,
        subscriber: &mut Subscriber<PublishEntryState>,
        client_message: ClientMessage,
    ) -> Result<(), WebSocketError> {
        let app_state = subscriber.app_state.clone();
        match client_message {
            ClientMessage::Login(login_message) => {
                // Check if this publisher already has an active session
                if let Some(mut session) = app_state
                    .publisher_sessions
                    .get_mut(&login_message.publisher_name)
                {
                    if session.is_expired() {
                        // Remove expired session
                        subscriber
                            .app_state
                            .publisher_sessions
                            .remove(&login_message.publisher_name);
                    } else {
                        // Reset the session login time
                        session.login_time = SystemTime::now();
                    }
                }

                let result = process_login(subscriber, login_message.clone()).await;
                let has_login_failed = result.is_err();
                let response = match result {
                    Ok(()) => {
                        // Store the new session with IP address
                        subscriber.app_state.publisher_sessions.insert(
                            login_message.publisher_name.clone(),
                            PublisherSession::new(subscriber.ip_address),
                        );
                        // Update subscriber state
                        {
                            let mut state = subscriber.state.write().await;
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
                    .send_msg(response)
                    .await
                    .map_err(|_| WebSocketError::ChannelClose)?;

                // If login was unsuccessful we just close the channel
                if has_login_failed {
                    return Err(WebSocketError::ChannelClose);
                }
            }
            ClientMessage::Publish(new_entries) => {
                // Check login state, session expiry and IP match
                let should_send_error = {
                    let state = subscriber.state.read().await;

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
                            } else if !session.validate_ip(&subscriber.ip_address) {
                                Some(PublishResponse {
                                    status: "error".to_string(),
                                    message: "Invalid IP address for this publisher session"
                                        .to_string(),
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
                        .send_msg(error_response.clone())
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
                    .send_msg(response)
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
            let state = subscriber.state.read().await;
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
                .send_msg(response)
                .await
                .map_err(|_| WebSocketError::ChannelClose)?;
            return Err(WebSocketError::ChannelClose);
        }
        Ok(())
    }
}

#[tracing::instrument(skip_all)]
async fn process_entries_without_verification(
    subscriber: &Subscriber<PublishEntryState>,
    new_entries: PublishEntryRequest,
) -> Result<CreateEntryResponse, EntryError> {
    tracing::debug!("Received new entries via WebSocket..");

    if new_entries.entries.is_empty() {
        return Ok(CreateEntryResponse {
            number_entries_created: 0,
        });
    }

    let publisher_name = &subscriber.state.read().await.publisher_name;
    let publisher_name = publisher_name.as_ref().ok_or_else(|| {
        EntryError::PublisherNotFound("No publisher name in session state".to_string())
    })?;

    // Split entries into spot and perp entries
    let (spot_entries, perp_entries): (Vec<_>, Vec<_>) = new_entries
        .entries
        .iter()
        .partition(|entry| matches!(entry, MarketEntry::Spot(_)));

    // Convert spot entries
    let spot_entries_db: Vec<NewEntry> = spot_entries
        .iter()
        .filter_map(|entry| {
            if let MarketEntry::Spot(entry) = entry {
                Some(convert_entry_to_db(
                    entry,
                    &Signature {
                        r: Felt::ZERO,
                        s: Felt::ZERO,
                    },
                ))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>, EntryError>>()?;

    // Convert perp entries
    let perp_entries_db: Vec<NewFutureEntry> = perp_entries
        .iter()
        .filter_map(|entry| {
            if let MarketEntry::Perp(entry) = entry {
                Some(convert_perp_entry_to_db(
                    entry,
                    &Signature {
                        r: Felt::ZERO,
                        s: Felt::ZERO,
                    },
                ))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>, EntryError>>()?;

    let config = crate::config::config().await;

    // Publish spot entries if any exist
    if !spot_entries_db.is_empty() {
        publish_to_kafka(
            spot_entries_db,
            config.kafka_topic().to_string(),
            publisher_name,
        )
        .await?;
    }

    // Publish perp entries if any exist
    if !perp_entries_db.is_empty() {
        publish_to_kafka(
            perp_entries_db,
            config.kafka_topic().to_string(),
            publisher_name,
        )
        .await?;
    }

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

    let publishers_cache = state.caches.publishers();
    let (public_key, account_address) =
        validate_publisher(&state.offchain_pool, &publisher_name, publishers_cache).await?;

    assert_login_is_valid(message, signature, &account_address, &public_key)
        .map_err(|_| EntryError::InvalidSigner)?;

    Ok(())
}
