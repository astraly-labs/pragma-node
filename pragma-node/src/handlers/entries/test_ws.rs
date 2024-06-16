use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tokio::time::{interval, Interval};
use uuid::Uuid;

use crate::AppState;

/// Subscriber is an actor that handles a single websocket connection.
/// It listens to the store for updates and sends them to the client.
pub struct Subscriber<WsState, Payload> {
    pub id: Uuid,
    pub ip_address: Option<IpAddr>,
    // TODO: what is the purpose on closed?
    pub closed: bool,
    pub _app_state: Arc<AppState>,
    pub _ws_state: Arc<WsState>,
    // TODO: what do I use this for?
    pub notify_receiver: Receiver<Payload>,
    pub sender: SplitSink<WebSocket, Message>,
    pub receiver: SplitStream<WebSocket>,
    pub update_interval: Interval,
    // TODO: watches for exit but where/when is it set?
    pub exit: watch::Receiver<bool>,
}

pub trait SubscriberHandler<WsState, Payload> {
    async fn handle_message(&mut self, message: Payload);
    async fn handle_payload(&mut self, payload: Payload);
    async fn decode_and_handle_message_received(
        &mut self,
        msg: Message,
        subscriber: &mut Subscriber<WsState, Payload>,
    );
    async fn periodic_interval(&mut self, subscriber: &mut Subscriber<WsState, Payload>);
}

impl<WsState, Payload> Subscriber<WsState, Payload>
where
    WsState: Default + std::fmt::Debug + Send + Sync + 'static,
    Payload: std::fmt::Debug + Send + Sync + 'static,
{
    pub fn new(socket: WebSocket, app_state: Arc<AppState>) -> Self {
        let (sender, receiver) = socket.split();
        let (_, rx) = mpsc::channel::<Payload>(1);
        let (_, exit_rx) = watch::channel(false);

        let init_state = WsState::default();

        Subscriber {
            id: Uuid::new_v4(),
            ip_address: None,
            closed: false,
            _app_state: app_state,
            _ws_state: Arc::new(init_state),
            notify_receiver: rx,
            sender,
            receiver,
            update_interval: interval(Duration::from_secs(1)),
            exit: exit_rx,
        }
    }

    pub async fn listen<H>(&mut self, mut handler: H)
    where
        H: SubscriberHandler<WsState, Payload> + Send + 'static,
    {
        loop {
            tokio::select! {
                maybe_msg = self.receiver.next() => {
                    match maybe_msg {
                        Some(Ok(msg)) => {
                            handler.decode_and_handle_message_received(msg, self).await;
                        }
                        Some(Err(_)) => {
                            tracing::info!("Client disconnected or error occurred. Closing the channel.");
                            break;
                        },
                        None => {}
                    }
                },
                // TODO: does not work as intended
                // maybe_payload = self.notify_receiver.recv() => {
                //     if let Some(payload) = maybe_payload {
                //         handler.handle_payload(payload).await;
                //     } else {
                //         tracing::error!("Could not receive payload");
                //     }
                // },
                _ = self.update_interval.tick() => {
                    handler.periodic_interval(self).await;
                },
                _ = self.exit.changed() => {
                    if *self.exit.borrow() {
                        tracing::info!("Exit signal received. Closing the channel.");
                        break;
                    }
                },
            }
        }
    }
}

// ==================================================

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
    async fn handle_message(&mut self, message: ChannelCommunication) {
        // Custom logic for handling a message
        tracing::info!("Handling message: {:?}", message);
    }

    async fn handle_payload(&mut self, payload: ChannelCommunication) {
        // Custom logic for handling a payload
        tracing::info!("Handling payload: {:?}", payload);
    }

    async fn decode_and_handle_message_received(
        &mut self,
        msg: Message,
        subscriber: &mut Subscriber<WsState, ChannelCommunication>,
    ) {
        match msg {
            Message::Text(text) => {
                tracing::info!("📨 [TEXT]");
                let msg = serde_json::from_str::<ChannelCommunication>(&text);
                if let Ok(msg) = msg {
                    self.handle_message(msg).await;
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
        tracing::info!("🕒 [PERIODIC INTERVAL]");
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
pub async fn test_ws(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| create_new_subscriber(socket, state))
}

async fn create_new_subscriber(socket: WebSocket, app_state: AppState) {
    let mut subscriber =
        Subscriber::<WsState, ChannelCommunication>::new(socket, Arc::new(app_state));
    let handler = WsTestHandler;
    let _ = subscriber
        .sender
        .send(Message::Text(format!(
            "You are registered as:\n[{:?}] {}",
            subscriber.ip_address, subscriber.id
        )))
        .await;
    subscriber.listen(handler).await;
}
