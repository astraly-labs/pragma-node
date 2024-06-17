use std::fmt::Debug;
use std::net::IpAddr;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;

use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::watch;
use tokio::time::{interval, Interval};
use uuid::Uuid;

use crate::AppState;

pub type SubscriberId = Uuid;

/// Subscriber is an actor that handles a single websocket connection.
/// It listens to the store for updates and sends them to the client.
pub struct Subscriber<WsState, Payload> {
    pub id: SubscriberId,
    pub ip_address: IpAddr,
    // TODO: what is the purpose on closed?
    pub closed: bool,
    pub _app_state: Arc<AppState>,
    pub _ws_state: Arc<WsState>,
    // server sending messages to the client
    pub sender: SplitSink<WebSocket, Message>,
    // server receiving messages from the client
    pub receiver: SplitStream<WebSocket>,
    pub update_interval: Interval,
    // client receiving messages from the server
    pub notify_receiver: Receiver<Payload>,
    // TODO: watches for exit but where/when is it set?
    pub exit_receiver: watch::Receiver<bool>,
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
    // TODO(akhercha): Remove Debug
    WsState: Default + Debug,
    Payload: Debug,
{
    pub async fn new(
        socket: WebSocket,
        ip_address: IpAddr,
        app_state: Arc<AppState>,
        update_interval_in_ms: u64,
        notify_receiver: Receiver<Payload>,
        exit_receiver: watch::Receiver<bool>,
    ) -> Result<Self, String> {
        let (sender, receiver) = socket.split();
        let init_state = WsState::default();

        let mut subscriber = Subscriber {
            id: Uuid::new_v4(),
            ip_address,
            closed: false,
            _app_state: app_state,
            _ws_state: Arc::new(init_state),
            sender,
            receiver,
            update_interval: interval(Duration::from_millis(update_interval_in_ms)),
            notify_receiver,
            exit_receiver,
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
                _ = self.exit_receiver.changed() => {
                    if *self.exit_receiver.borrow() {
                        tracing::info!("⛔ [CLOSING SIGNAL]");
                        break;
                    }
                },
            }
        }
    }
}
