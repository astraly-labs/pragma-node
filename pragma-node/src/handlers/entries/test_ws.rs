use std::fmt::Display;
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

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WsState {
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ChannelCommunication {
    pub msg: String,
}

/// Subscriber is an actor that handles a single websocket connection.
/// It listens to the store for updates and sends them to the client.
pub struct Subscriber<WsState, Payload> {
    pub id: Uuid,
    pub ip_address: Option<IpAddr>,
    pub closed: bool,
    pub _app_state: Arc<AppState>,
    pub ws_state: Arc<WsState>,
    // TODO: what do I use this for?
    pub _notify_receiver: Receiver<Payload>,
    pub sender: SplitSink<WebSocket, Message>,
    pub receiver: SplitStream<WebSocket>,
    pub update_interval: Interval,
    pub exit: watch::Receiver<bool>,
}

impl Display for Subscriber<WsState, ChannelCommunication> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Subscriber {{ id: {}, ip_address: {:?}, closed: {}, ws_state: {:?} }}",
            self.id, self.ip_address, self.closed, self.ws_state
        )
    }
}

impl<WsState: Default, Payload> Subscriber<WsState, Payload> {
    pub fn new(socket: WebSocket, app_state: Arc<AppState>) -> Self {
        let (sender, receiver) = socket.split();
        let (_, rx) = mpsc::channel::<Payload>(32);
        let (_, exit_rx) = watch::channel(false);

        let init_state = WsState::default();

        Subscriber {
            id: Uuid::new_v4(),
            ip_address: None,
            closed: false,
            _app_state: app_state,
            ws_state: Arc::new(init_state),
            _notify_receiver: rx,
            sender,
            receiver,
            update_interval: interval(Duration::from_secs(1)),
            exit: exit_rx,
        }
    }

    pub async fn listen(&mut self) {
        loop {
            tokio::select! {
                maybe_msg = self.receiver.next() => {
                    match maybe_msg {
                        Some(Ok(msg)) => {
                            self.decode_and_handle_message_received(msg).await;
                        }
                        Some(Err(_)) => {
                            tracing::info!("Client disconnected or error occurred. Closing the channel.");
                            break;
                        },
                        None => {}
                    }
                },
                // maybe_payload = self.notify_receiver.recv() => {
                //     match maybe_payload {
                //         Some(payload) => {
                //             self.handle_payload(payload).await;
                //         }
                //         None => {
                //             tracing::info!("Nuhhh");
                //         }
                //     }
                // },
                _ = self.update_interval.tick() => {
                    self.periodic_interval().await;
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

    async fn decode_and_handle_message_received(&mut self, msg: Message) {
        match msg {
            Message::Text(_) => {
                tracing::info!("[TEXT]");
            }
            Message::Binary(_) => {
                tracing::info!("[BINARY]");
            }
            Message::Ping(_) => {
                tracing::info!("[PING]");
            }
            Message::Pong(_) => {
                tracing::info!("[PONG]");
            }
            Message::Close(_) => {
                tracing::info!("[CLOSING]");
                self.closed = true;
            }
        }
    }

    async fn _handle_message(&mut self, _message: Payload) {
        todo!()
    }

    async fn _handle_payload(&mut self, _payload: Payload) {
        todo!()
    }

    async fn periodic_interval(&mut self) {
        if self.closed {
            return;
        }
        let _ = self.sender.send(Message::Text("tic".to_string())).await;
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

/// Handle the WebSocket channel.
async fn create_new_subscriber(socket: WebSocket, app_state: AppState) {
    let mut subscriber =
        Subscriber::<WsState, ChannelCommunication>::new(socket, Arc::new(app_state));
    let _ = subscriber
        .sender
        .send(Message::Text(format!(
            "You are registered as:\n{}",
            subscriber
        )))
        .await;
    subscriber.listen().await;
}
