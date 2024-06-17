use std::net::IpAddr;
use std::net::SocketAddr;
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

pub trait SubscriberHandler<WsState, Payload> {
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
    );
    async fn periodic_interval(&mut self, subscriber: &mut Subscriber<WsState, Payload>);
}

impl<WsState, Payload> Subscriber<WsState, Payload>
where
    WsState: Default + std::fmt::Debug + Send + Sync + 'static,
    Payload: std::fmt::Debug + Send + Sync + 'static,
{
    pub fn new(
        socket: WebSocket,
        ip_address: IpAddr,
        app_state: Arc<AppState>,
        notify_receiver: Receiver<Payload>,
        update_interval_in_ms: u64,
    ) -> Self {
        let (sender, receiver) = socket.split();
        let (_, exit_rx) = watch::channel(false);
        let init_state = WsState::default();

        Subscriber {
            id: Uuid::new_v4(),
            ip_address,
            closed: false,
            _app_state: app_state,
            _ws_state: Arc::new(init_state),
            notify_receiver,
            sender,
            receiver,
            update_interval: interval(Duration::from_millis(update_interval_in_ms)),
            exit: exit_rx,
        }
    }

    pub async fn listen<H>(&mut self, mut handler: H)
    where
        H: SubscriberHandler<WsState, Payload> + Send + 'static,
    {
        loop {
            tokio::select! {
                // Messages from the client
                maybe_msg = self.receiver.next() => {
                    match maybe_msg {
                        Some(Ok(msg)) => {
                            handler.decode_and_handle_message_received(self, msg).await;
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
                // TODO: implementing this block stops the periodic intervals?
                // Some(payload) = self.notify_receiver.recv() => {
                //     tracing::info!("🥡 [SERVER PAYLOAD]");
                //     handler.handle_server_message(self, payload).await;
                // },
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

struct WsTestHandler;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WsState {
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ChannelCommunication {
    pub msg: String,
}

impl SubscriberHandler<WsState, ChannelCommunication> for WsTestHandler {
    async fn handle_client_message(&mut self, message: ChannelCommunication) {
        tracing::info!("{:?}", message);
    }

    async fn handle_server_message(
        &mut self,
        subscriber: &mut Subscriber<WsState, ChannelCommunication>,
        message: ChannelCommunication,
    ) {
        let _ = subscriber
            .sender
            .send(Message::Text(serde_json::to_string(&message).unwrap()))
            .await;
    }

    async fn decode_and_handle_message_received(
        &mut self,
        subscriber: &mut Subscriber<WsState, ChannelCommunication>,
        msg: Message,
    ) {
        match msg {
            Message::Text(text) => {
                tracing::info!("📨 [TEXT]");
                let msg = serde_json::from_str::<ChannelCommunication>(&text);
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
            Message::Close(_) => {
                tracing::info!("👋 [CLOSE]");
                subscriber.closed = true;
            }
        }
    }

    async fn periodic_interval(
        &mut self,
        subscriber: &mut Subscriber<WsState, ChannelCommunication>,
    ) {
        if subscriber.closed {
            // If the channel is closed, we shouldn't do anything here
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
    tracing::info!("🔗 [IP] {:?}", client_addr);
    ws.on_upgrade(move |socket| create_new_subscriber(socket, state, client_addr))
}

async fn create_new_subscriber(socket: WebSocket, app_state: AppState, client_addr: SocketAddr) {
    // Channel communication between the server & the subscriber
    let (notify_sender, notify_receiver) = mpsc::channel::<ChannelCommunication>(100);

    let mut subscriber = Subscriber::<WsState, ChannelCommunication>::new(
        socket,
        client_addr.ip(),
        Arc::new(app_state),
        notify_receiver,
        1000,
    );

    let handler = WsTestHandler;
    let _ = subscriber
        .sender
        .send(Message::Text(format!(
            "You are registered as:\n[{:?}] {}",
            subscriber.ip_address, subscriber.id
        )))
        .await;

    tokio::spawn(async move {
        subscriber.listen(handler).await;
    });

    notify_sender
        .send(ChannelCommunication {
            msg: String::from("Hello from the server"),
        })
        .await
        .unwrap();
}
