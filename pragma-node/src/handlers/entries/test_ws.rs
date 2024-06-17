use std::net::IpAddr;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tokio::time::{interval, Interval};
use uuid::Uuid;

use crate::AppState;

// ====================================================================================
// ====================================================================================
// ====================================================================================

/// Subscriber is an actor that handles a single websocket connection.
/// It listens to the store for updates and sends them to the client.
pub struct Subscriber<WsState, Payload> {
    pub id: Uuid,
    pub ip_address: IpAddr,
    // TODO: what is the purpose on closed?
    pub closed: bool,
    pub _app_state: Arc<AppState>,
    pub _ws_state: Arc<WsState>,
    // client receiving messages from the server
    pub notify_receiver: Receiver<Payload>,
    // server sending messages to the client
    pub sender: SplitSink<WebSocket, Message>,
    // server receiving messages from the client
    pub receiver: SplitStream<WebSocket>,
    pub update_interval: Interval,
    // TODO: watches for exit but where/when is it set?
    pub exit: watch::Receiver<bool>,
}

pub trait ChannelHandler<WsState, Payload> {
    async fn handle_client_message(&mut self, message: Payload);
    async fn handle_server_message(
        &mut self,
        subscriber: &mut Subscriber<WsState, Payload>,
        payload: Payload,
    );
    async fn decode_and_handle_message_received(
        &mut self,
        subscriber: &mut Subscriber<WsState, Payload>,
        msg: Message,
    ) -> ControlFlow<(), ()>;
    async fn periodic_interval(&mut self, subscriber: &mut Subscriber<WsState, Payload>);
}

impl<WsState, Payload> Subscriber<WsState, Payload>
where
    WsState: Default + std::fmt::Debug + Send + Sync + 'static,
    Payload: std::fmt::Debug + Send + Sync + 'static,
{
    pub async fn new(
        socket: WebSocket,
        ip_address: IpAddr,
        app_state: Arc<AppState>,
        notify_receiver: Receiver<Payload>,
        exit_receiver: watch::Receiver<bool>,
        update_interval_in_ms: u64,
    ) -> Result<Self, String> {
        let (sender, receiver) = socket.split();
        let init_state = WsState::default();

        let mut subscriber = Subscriber {
            id: Uuid::new_v4(),
            ip_address,
            closed: false,
            _app_state: app_state,
            _ws_state: Arc::new(init_state),
            notify_receiver,
            sender,
            receiver,
            update_interval: interval(Duration::from_millis(update_interval_in_ms)),
            exit: exit_receiver,
        };

        subscriber.assert_is_healthy().await?;

        Ok(subscriber)
    }

    /// Perform the initial handshake with the client - ensure the channel is healthy
    async fn assert_is_healthy(&mut self) -> Result<(), String> {
        let ping_status = self.sender.send(Message::Ping(vec![1, 2, 3])).await;
        if ping_status.is_err() {
            return Err("Failed to send a ping message".to_string());
        }
        Ok(())
    }

    pub async fn listen<H>(&mut self, mut handler: H)
    where
        H: ChannelHandler<WsState, Payload>,
    {
        loop {
            tokio::select! {
                // Messages from the client
                maybe_client_msg = self.receiver.next() => {
                    match maybe_client_msg {
                        Some(Ok(client_msg)) => {
                            tracing::info!("New message from client 👇");
                            let client_msg = handler.decode_and_handle_message_received(self, client_msg).await;
                            if client_msg.is_break() {
                                break;
                            }
                        }
                        Some(Err(_)) => {
                            tracing::info!("Client disconnected or error occurred. Closing the channel.");
                            break;
                        },
                        None => {}
                    }
                },
                // Periodic updates
                _ = self.update_interval.tick() => {
                    tracing::info!("🕒 [PERIODIC INTERVAL]");
                    handler.periodic_interval(self).await;
                },
                // Messages from the server to the client
                maybe_server_msg = self.notify_receiver.recv() => {
                    if let Some(server_msg) = maybe_server_msg {
                        tracing::info!("🥡 [SERVER MESSAGE]");
                        handler.handle_server_message(self, server_msg).await;
                    }
                },
                // Exit signal
                _ = self.exit.changed() => {
                    if *self.exit.borrow() {
                        tracing::info!("⛔ [CLOSING SIGNAL]");
                        break;
                    }
                },
            }
        }
    }
}

// ====================================================================================
// ====================================================================================
// ====================================================================================

struct WsTestEndpointHandler;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WsState {
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ChannelUpdateMsg {
    pub msg: String,
}

impl ChannelHandler<WsState, ChannelUpdateMsg> for WsTestEndpointHandler {
    async fn handle_client_message(&mut self, message: ChannelUpdateMsg) {
        tracing::info!("{:?}", message);
    }

    async fn handle_server_message(
        &mut self,
        subscriber: &mut Subscriber<WsState, ChannelUpdateMsg>,
        message: ChannelUpdateMsg,
    ) {
        let _ = subscriber
            .sender
            .send(Message::Text(serde_json::to_string(&message).unwrap()))
            .await;
    }

    // Decode & handle messages received from the client
    async fn decode_and_handle_message_received(
        &mut self,
        subscriber: &mut Subscriber<WsState, ChannelUpdateMsg>,
        msg: Message,
    ) -> ControlFlow<(), ()> {
        match msg {
            Message::Close(_) => {
                tracing::info!("👋 [CLOSE]");
                subscriber.closed = true;
                return ControlFlow::Break(());
            }
            Message::Text(text) => {
                tracing::info!("📨 [TEXT]");
                let msg = serde_json::from_str::<ChannelUpdateMsg>(&text);
                if let Ok(msg) = msg {
                    self.handle_client_message(msg).await;
                } else {
                    tracing::error!("Could not decode message");
                }
            }
            Message::Binary(_) => {
                tracing::info!("📨 [BINARY]");
            }
            Message::Ping(_) => {
                tracing::info!("📨 [PING]");
            }
            Message::Pong(_) => {
                tracing::info!("📨 [PONG]");
            }
        }
        ControlFlow::Continue(())
    }

    async fn periodic_interval(&mut self, subscriber: &mut Subscriber<WsState, ChannelUpdateMsg>) {
        if subscriber.closed {
            return;
        }
        let _ = subscriber
            .sender
            .send(Message::Text("tic".to_string()))
            .await;
    }
}

// ====================================================================================
// ====================================================================================
// ====================================================================================

#[utoipa::path(
    get,
    path = "/node/v1/data/test",
    responses(
        (
            status = 200,
            description = "Subscribe to a list of entries",
            body = [SubscribeToEntryResponse]
        )
    )
)]
pub async fn test_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| create_new_subscriber(socket, state, client_addr))
}

async fn create_new_subscriber(socket: WebSocket, app_state: AppState, client_addr: SocketAddr) {
    // Channel communication between the server & the subscriber
    let (notify_sender, notify_receiver) = mpsc::channel::<ChannelUpdateMsg>(100);
    // Exit signal, allow to close the channel from the server side
    let (exit_sender, exit_receiver) = watch::channel(false);

    let mut subscriber = match Subscriber::<WsState, ChannelUpdateMsg>::new(
        socket,
        client_addr.ip(),
        Arc::new(app_state),
        notify_receiver,
        exit_receiver,
        1000,
    )
    .await
    {
        Ok(subscriber) => subscriber,
        Err(e) => {
            tracing::error!("Failed to create a new subscriber. Error: {}", e);
            return;
        }
    };

    // Save information about the current subscriber
    let current_subscriber_id = subscriber.id;
    let current_subscriber_ip = subscriber.ip_address;

    // Send a welcome message
    let _ = subscriber
        .sender
        .send(Message::Text(format!(
            "You are registered as:\n[{:?}] {}",
            subscriber.ip_address, subscriber.id
        )))
        .await;

    // Main event loop for the subscriber
    let handler = WsTestEndpointHandler;
    tokio::spawn(async move {
        subscriber.listen(handler).await;
    });

    // Send some messages to the client as the server using the notification chan
    for _ in 0..50 {
        let _ = notify_sender
            .send(ChannelUpdateMsg {
                msg: String::from("Hello from the server"),
            })
            .await;
        // wait 5s
        tokio::time::sleep(Duration::from_secs(10)).await;
    }

    // close the channel when talk is over
    tracing::info!(
        "End of discussions. Closing channel for [{}] {}",
        current_subscriber_ip,
        current_subscriber_id
    );
    let _ = exit_sender.send(true);
}
