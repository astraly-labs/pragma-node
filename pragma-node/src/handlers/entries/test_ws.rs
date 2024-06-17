use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};

use crate::types::ws::{ChannelHandler, Subscriber, WebSocketError};
use crate::AppState;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WsState {
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ClientMsg {
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ServerMsg {
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PriceUpdate {
    new_price: String,
}

struct WsTestHandler;
impl ChannelHandler<WsState, ClientMsg, ServerMsg> for WsTestHandler {
    async fn handle_client_msg(
        &mut self,
        _subscriber: &mut Subscriber<WsState>,
        message: ClientMsg,
    ) -> Result<(), WebSocketError> {
        tracing::info!("{:?}", message);
        Ok(())
    }

    async fn handle_server_msg(
        &mut self,
        subscriber: &mut Subscriber<WsState>,
        message: ServerMsg,
    ) -> Result<(), WebSocketError> {
        let _ = subscriber
            .sender
            .send(Message::Text(serde_json::to_string(&message).unwrap()))
            .await;
        Ok(())
    }

    // Decode & handle messages received from the client
    async fn decode_client_msg(
        &mut self,
        subscriber: &mut Subscriber<WsState>,
        msg: Message,
    ) -> Option<ClientMsg> {
        match msg {
            Message::Close(_) => {
                tracing::info!("👋 [CLOSE]");
                let _ = subscriber.exit.0.send(true);
                return None;
            }
            Message::Text(text) => {
                tracing::info!("📨 [TEXT]");
                let msg = serde_json::from_str::<ClientMsg>(&text);
                if let Ok(msg) = msg {
                    return Some(msg);
                } else {
                    tracing::error!("😱 Could not decode message from client");
                    return None;
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
        None
    }

    async fn decode_server_msg(
        &mut self,
        _subscriber: &mut Subscriber<WsState>,
        msg: Message,
    ) -> Option<ServerMsg> {
        match msg {
            Message::Close(_) => {
                tracing::info!("👋 [CLOSE]");
                // Shouldn't do anything for the client
                return None;
            }
            Message::Text(text) => {
                tracing::info!("📨 [TEXT]");
                let msg = serde_json::from_str::<ServerMsg>(&text);
                if let Ok(msg) = msg {
                    return Some(msg);
                } else {
                    tracing::error!("😱 Could not decode message from server");
                    return None;
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
        None
    }

    async fn periodic_interval(
        &mut self,
        subscriber: &mut Subscriber<WsState>,
    ) -> Result<(), WebSocketError> {
        if subscriber.closed {
            return Ok(());
        }
        if let Ok(msg) = serde_json::to_string(&PriceUpdate {
            new_price: "100".to_string(),
        }) {
            let _ = subscriber.sender.send(Message::Text(msg)).await;
        }
        Ok(())
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
pub async fn test_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| create_new_subscriber(socket, state, client_addr))
}

async fn create_new_subscriber(socket: WebSocket, app_state: AppState, client_addr: SocketAddr) {
    // Channel communication between the server & the subscriber
    let (mut subscriber, notify_sender) =
        match Subscriber::<WsState>::new(socket, client_addr.ip(), Arc::new(app_state), 1000).await
        {
            Ok(subscriber) => subscriber,
            Err(e) => {
                tracing::error!("Failed to create a new subscriber. Error: {}", e);
                return;
            }
        };

    // Send a welcome message
    let _ = notify_sender
        .send(Message::Text(
            serde_json::to_string(&ServerMsg {
                msg: format!(
                    "You are registered as:\n[{:?}] {}",
                    subscriber.ip_address, subscriber.id
                ),
            })
            .unwrap(),
        ))
        .await;

    // Main event loop for the subscriber
    let handler = WsTestHandler;
    tokio::spawn(async move {
        let _ = subscriber.listen(handler).await;
    });

    // Send a message every 10s to the subscriber
    for _ in 0..10 {
        let _ = notify_sender
            .send(Message::Text(
                serde_json::to_string(&ServerMsg {
                    msg: "Hello from the server".to_string(),
                })
                .unwrap(),
            ))
            .await;
        // sleep for 10s
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
