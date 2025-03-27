use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use futures::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use governor::{Quota, RateLimiter};
use nonzero_ext::nonzero;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{RwLock, mpsc, watch};
use tokio::time::Interval;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::metrics::{Interaction, Status};
use crate::{metrics, state::AppState};

#[derive(Default, Debug, Serialize, Deserialize)]
pub enum SubscriptionType {
    #[serde(rename = "subscribe")]
    #[default]
    Subscribe,
    #[serde(rename = "unsubscribe")]
    Unsubscribe,
}

#[derive(Debug, thiserror::Error)]
pub enum WebSocketError {
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Failed to send message")]
    SendError(#[from] mpsc::error::SendError<Message>),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Message serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Failed to decode message: {0}")]
    DecodingError(String),
}

// Subscriber struct managing WebSocket connections
pub struct Subscriber<ChannelState> {
    pub id: Uuid,
    pub state: Arc<RwLock<ChannelState>>,
    pub app_state: Arc<AppState>,
    endpoint_name: String,
    pub ip_address: IpAddr,
    server_msg_sender: mpsc::Sender<Message>,
    client_msg_receiver: mpsc::Receiver<Message>,
    update_interval: Interval,
    rate_limiter: RateLimiter<
        IpAddr,
        governor::state::keyed::DefaultKeyedStateStore<IpAddr>,
        governor::clock::DefaultClock,
    >,
    message_count_limiter: RateLimiter<
        IpAddr,
        governor::state::keyed::DefaultKeyedStateStore<IpAddr>,
        governor::clock::DefaultClock,
    >,
    exit: (
        tokio::sync::watch::Sender<bool>,
        tokio::sync::watch::Receiver<bool>,
    ),
    last_activity: std::time::Instant,
    tasks_cancellation: CancellationToken,
}

#[async_trait::async_trait]
pub trait ChannelHandler<ChannelState, CM, Err> {
    /// Called after a message is received from the client.
    /// The handler should process the message and update the state.
    async fn handle_client_msg(
        &mut self,
        subscriber: &mut Subscriber<ChannelState>,
        message: CM,
    ) -> Result<(), Err>;

    /// Called at a regular interval to update the client with the latest state.
    async fn periodic_interval(
        &mut self,
        subscriber: &mut Subscriber<ChannelState>,
    ) -> Result<(), Err>;
}

impl<ChannelState> Subscriber<ChannelState>
where
    ChannelState: Default + Send + Sync + 'static,
{
    /// Creates a new Subscriber instance tied to a WebSocket connection.
    ///
    /// # Arguments
    /// - `endpoint_name`: Name of the endpoint (e.g., `subscribe_to_onchain_ohlc`).
    /// - `socket`: The WebSocket connection.
    /// - `ip_address`: Client's IP address for rate limiting.
    /// - `app_state`: Shared application state.
    /// - `state`: Optional initial channel state.
    /// - `update_interval_in_ms`: Interval (in milliseconds) for periodic updates.
    /// - `rate_limit_quota`: Configurable rate limit quota for this endpoint.
    ///
    /// # Returns
    /// A tuple containing the Subscriber and a Sender for sending messages to the client.
    pub fn new(
        endpoint_name: String,
        socket: WebSocket,
        ip_address: IpAddr,
        app_state: Arc<AppState>,
        state: Option<ChannelState>,
        update_interval_in_ms: u64,
        rate_limit_quota: Option<Quota>,
    ) -> Result<Self, WebSocketError> {
        /// The maximum number of bytes that can be sent per second per IP address.
        /// If the limit is exceeded, the connection is closed.
        const BYTES_LIMIT_PER_IP_PER_SECOND: u32 = 256 * 1024; // 256 KiB
        /// The maximum number of messages send-able per second.
        const MESSAGES_LIMIT_PER_IP_PER_SECOND: u32 = 64;

        let id = Uuid::new_v4();
        let (ws_sender, ws_receiver) = socket.split();
        let (server_msg_sender, server_msg_receiver) = mpsc::channel::<Message>(128);
        let (client_msg_sender, client_msg_receiver) = mpsc::channel::<Message>(128);

        let rate_limit_quota =
            rate_limit_quota.unwrap_or(Quota::per_second(nonzero!(BYTES_LIMIT_PER_IP_PER_SECOND)));
        let msg_limit_quota = Quota::per_second(nonzero!(MESSAGES_LIMIT_PER_IP_PER_SECOND));

        // Spawn sending and receiving tasks
        let cancellation_token = Self::spawn_ws_tasks(
            ws_sender,
            ws_receiver,
            server_msg_receiver,
            client_msg_sender,
        );

        let subscriber = Self {
            id,
            state: Arc::new(RwLock::new(state.unwrap_or_default())),
            app_state,
            endpoint_name,
            ip_address,
            server_msg_sender,
            client_msg_receiver,
            update_interval: tokio::time::interval(Duration::from_millis(update_interval_in_ms)),
            rate_limiter: RateLimiter::dashmap(rate_limit_quota),
            message_count_limiter: RateLimiter::dashmap(msg_limit_quota),
            exit: watch::channel(false),
            last_activity: std::time::Instant::now(),
            tasks_cancellation: cancellation_token,
        };

        // Retain the recent rate limit data for the IP addresses to
        // prevent the rate limiter size from growing indefinitely.
        subscriber.rate_limiter.retain_recent();

        subscriber.record_metric(
            metrics::Interaction::NewConnection,
            metrics::Status::Success,
        );

        Ok(subscriber)
    }

    /// Spawns WebSocket tasks and returns a cancellation token.
    ///
    /// The tasks are responsible for sending & receiving message for the socket
    /// that we after forward to the Subscriber.
    fn spawn_ws_tasks(
        mut ws_sender: SplitSink<WebSocket, Message>,
        mut ws_receiver: SplitStream<WebSocket>,
        mut server_msg_receiver: mpsc::Receiver<Message>,
        client_msg_sender: mpsc::Sender<Message>,
    ) -> CancellationToken {
        let token = CancellationToken::new();
        let task_token = token.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle cancellation
                    () = task_token.cancelled() => {
                        let _ = ws_sender.close().await;
                        break;
                    }

                    // Send messages from server to client
                    Some(msg) = server_msg_receiver.recv() => {
                        if ws_sender.send(msg).await.is_err() {
                            break;
                        }
                    }

                    // Receive messages from client to server
                    Some(result) = ws_receiver.next() => {
                        match result {
                            Ok(msg) => {
                                if client_msg_sender.send(msg).await.is_err() {
                                    break;
                                }
                            }
                            Err(_) => {
                                break;
                            }
                        }
                    }

                    // Ensure we don’t spin if no messages are available
                    else => break,
                }
            }
        });

        token
    }

    /// Listens for client messages and invokes the handler periodically.
    pub async fn listen<H, CM, Err>(&mut self, mut handler: H) -> Result<(), Err>
    where
        H: ChannelHandler<ChannelState, CM, Err>,
        CM: for<'a> Deserialize<'a>,
    {
        const INACTIVITY_CHECK_INTERVAL: Duration = Duration::from_secs(20);
        let mut inactivity_timer = tokio::time::interval(INACTIVITY_CHECK_INTERVAL);

        loop {
            tokio::select! {
                // Check for inactivity
                _ = inactivity_timer.tick() => {
                    if self.is_inactive() {
                        self.send_err("Connection timeout due to inactivity").await;
                        self.server_msg_sender.send(Message::Close(None)).await.ok();
                        if self.exit.0.send(true).is_err() {
                            self.record_metric(Interaction::CloseConnection, Status::Error);
                        } else {
                            self.record_metric(Interaction::CloseConnection, Status::Success);
                        }
                        return Ok(());
                    }
                },

                // Messages from the client
                Some(client_msg) = self.client_msg_receiver.recv() => {
                    // Check message frequency rate limit
                    if self.message_count_limiter.check_key(&self.ip_address).is_err() {
                        self.send_err("Too many messages. Please slow down.").await;
                        continue;
                    }

                    handler = self.decode_and_handle(handler, client_msg).await?;
                },

                // Periodic updates in the channel
                _ = self.update_interval.tick() => {
                    let status = handler.periodic_interval(self).await;
                    match status {
                        Ok(()) => {
                            self.record_metric(Interaction::ChannelUpdate, Status::Success);
                        },
                        Err(e) => {
                            self.record_metric(Interaction::ChannelUpdate, Status::Error);
                            self.record_metric(Interaction::CloseConnection, Status::Success);
                            return Err(e);
                        }
                    }
                }

                // Check if the channel has been closed
                _ = self.exit.1.changed() => {
                    if *self.exit.1.borrow() {
                        self.record_metric(Interaction::CloseConnection, Status::Success);
                        return Ok(());
                    }
                },
            }
        }
    }

    /// Called after a message is received from the client.
    /// The handler should process the message and update the state.
    /// If the handler returns an error, the connection will be closed.
    async fn decode_and_handle<H, CM, Err>(
        &mut self,
        mut handler: H,
        client_msg: Message,
    ) -> Result<H, Err>
    where
        H: ChannelHandler<ChannelState, CM, Err>,
        CM: for<'a> Deserialize<'a>,
    {
        // Return early if the message could not be decoded
        let Ok(Some(client_msg)) = self.decode_msg::<CM>(client_msg).await else {
            self.record_metric(Interaction::ClientMessageDecode, Status::Error);
            return Ok(handler);
        };

        // Else, handle it
        self.record_metric(Interaction::ClientMessageDecode, Status::Success);
        let status = handler.handle_client_msg(self, client_msg).await;
        match status {
            Ok(()) => {
                self.record_metric(Interaction::ClientMessageProcess, Status::Success);
            }
            Err(e) => {
                self.record_metric(Interaction::ClientMessageProcess, Status::Error);
                self.record_metric(Interaction::CloseConnection, Status::Success);
                return Err(e);
            }
        }

        Ok(handler)
    }

    /// Decode the message into the expected type.
    ///
    /// If the message is not in the expected format, it will return None.
    /// If the message is a close signal, it will return None and send a close signal to the client.
    async fn decode_msg<T: for<'a> Deserialize<'a>>(
        &mut self,
        msg: Message,
    ) -> Result<Option<T>, WebSocketError> {
        match msg {
            Message::Close(_) => {
                if self.exit.0.send(true).is_err() {
                    self.record_metric(Interaction::CloseConnection, Status::Error);
                }
            }

            Message::Text(text) => {
                self.assert_client_message_size(text.len()).await?;

                match serde_json::from_str::<T>(&text) {
                    Ok(msg) => {
                        self.last_activity = std::time::Instant::now();
                        return Ok(Some(msg));
                    }
                    Err(e) => {
                        self.send_err("Error parsing JSON into valid websocket request.")
                            .await;
                        return Err(WebSocketError::DecodingError(e.to_string()));
                    }
                }
            }

            Message::Binary(payload) => {
                self.assert_client_message_size(payload.len()).await?;

                match serde_json::from_slice::<T>(&payload) {
                    Ok(msg) => {
                        self.last_activity = std::time::Instant::now();
                        return Ok(Some(msg));
                    }
                    Err(e) => {
                        self.send_err("Error parsing JSON into valid websocket request.")
                            .await;
                        return Err(WebSocketError::DecodingError(e.to_string()));
                    }
                }
            }

            Message::Ping(payload) => {
                self.last_activity = std::time::Instant::now();
                let _ = self.server_msg_sender.send(Message::Pong(payload)).await;
            }

            Message::Pong(_) => {}
        }
        Ok(None)
    }

    /// Sends a message to the client after checking the rate limit.
    pub async fn send_msg<T>(&mut self, message: T) -> Result<(), WebSocketError>
    where
        T: Sized + Serialize,
    {
        let message = serde_json::to_string(&message).map_err(WebSocketError::Serialization)?;

        let message_size = message.len();
        self.check_rate_limit(message_size).await?;
        self.server_msg_sender
            .send(Message::Text(message.into()))
            .await
            .map_err(WebSocketError::SendError)?;

        Ok(())
    }

    /// Sends an error message to the client.
    pub async fn send_err(&self, msg: &str) {
        let err = json!({
            "status": "error",
            "error": msg,
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });
        let _ = self
            .server_msg_sender
            .send(Message::Text(err.to_string().into()))
            .await;
    }

    /// Checks the rate limit for the given message size.
    ///
    /// If the limit is exceeded, it calls `handle_rate_limit_exceeded` to close the connection.
    async fn check_rate_limit(&self, message_size: usize) -> Result<(), WebSocketError> {
        let burst_size = NonZeroU32::new(message_size as u32)
            .ok_or(WebSocketError::InternalError("Invalid message size".into()))?;

        if self.rate_limiter.check_key_n(&self.ip_address, burst_size) != Ok(Ok(())) {
            self.handle_rate_limit_exceeded().await?;
            return Err(WebSocketError::RateLimitExceeded);
        }
        Ok(())
    }

    /// Handles the case when the rate limit is exceeded.
    ///
    /// Sends an error message to the client and closes the connection.
    async fn handle_rate_limit_exceeded(&self) -> Result<(), WebSocketError> {
        self.record_metric(metrics::Interaction::RateLimit, metrics::Status::Error);
        self.send_err("Rate limit exceeded. Closing connection.")
            .await;

        self.server_msg_sender
            .send(Message::Close(None))
            .await
            .map_err(WebSocketError::SendError)?;

        if self.exit.0.send(true).is_err() {
            self.record_metric(Interaction::CloseConnection, Status::Error);
        }
        Ok(())
    }

    async fn assert_client_message_size(&self, len: usize) -> Result<(), WebSocketError> {
        const MAX_MESSAGE_SIZE: usize = 1_048_576; // 1MB limit

        if len > MAX_MESSAGE_SIZE {
            let err = "Message too large.";
            self.send_err(err).await;
            return Err(WebSocketError::DecodingError(err.into()));
        }

        Ok(())
    }

    /// Checks if the client is inactive.
    ///
    /// A client is considered inactive after 30s without any message.
    fn is_inactive(&self) -> bool {
        const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(30);
        self.last_activity.elapsed() > INACTIVITY_TIMEOUT
    }

    /// Records a metric for the subscriber's interactions.
    pub fn record_metric(&self, interaction: Interaction, status: Status) {
        self.app_state.metrics.ws_metrics.record_ws_interaction(
            &self.endpoint_name,
            interaction,
            status,
        );
    }
}

// Cancel all tasks when subscriber is dropped
impl<ChannelState> Drop for Subscriber<ChannelState> {
    fn drop(&mut self) {
        self.tasks_cancellation.cancel();
    }
}
