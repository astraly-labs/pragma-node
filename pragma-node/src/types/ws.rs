use std::fmt::Debug;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;

use crate::AppState;
use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use thiserror::Error;
use tokio::sync::watch;
use tokio::time::{interval, Interval};
use uuid::Uuid;

pub type SubscriberId = Uuid;

#[derive(Debug, Error)]
pub enum WebSocketError {
    #[error("could not create a channel with the client")]
    ChannelInitError,
}

/// Subscriber is an actor that handles a single websocket connection.
/// It listens to the store for updates and sends them to the client.
pub struct Subscriber<WsState> {
    pub id: SubscriberId,
    pub ip_address: IpAddr,
    // TODO: what is the purpose on closed?
    pub closed: bool,
    pub _app_state: Arc<AppState>,
    pub _ws_state: Arc<WsState>,
    // server sending messages to the client
    pub channel: (SplitSink<WebSocket, Message>, SplitStream<WebSocket>),
    // server receiving messages from the client
    pub update_interval: Interval,
    // client receiving messages from the server
    pub notify_receiver: Receiver<Message>,
    // TODO: watches for exit but where/when is it set?
    pub exit: (watch::Sender<bool>, watch::Receiver<bool>),
}

pub trait ChannelHandler<WsState, CommMsg> {
    /// TODO.
    async fn decode_client_msg(
        &mut self,
        subscriber: &mut Subscriber<WsState>,
        msg: Message,
    ) -> Option<CommMsg>;

    /// TODO
    async fn handle_client_msg(
        &mut self,
        subscriber: &mut Subscriber<WsState>,
        message: CommMsg,
    ) -> Result<(), WebSocketError>;

    /// TODO.
    async fn decode_server_msg(
        &mut self,
        subscriber: &mut Subscriber<WsState>,
        msg: Message,
    ) -> Option<CommMsg>;

    /// TODO.
    async fn handle_server_msg(
        &mut self,
        subscriber: &mut Subscriber<WsState>,
        message: CommMsg,
    ) -> Result<(), WebSocketError>;

    /// TODO.
    async fn periodic_interval(
        &mut self,
        subscriber: &mut Subscriber<WsState>,
    ) -> Result<(), WebSocketError>;
}

impl<WsState> Subscriber<WsState>
where
    WsState: Default + Debug,
{
    pub async fn new(
        socket: WebSocket,
        ip_address: IpAddr,
        app_state: Arc<AppState>,
        update_interval_in_ms: u64,
        notify_receiver: Receiver<Message>,
    ) -> Result<Self, WebSocketError> {
        let init_state = WsState::default();

        let mut subscriber = Subscriber {
            id: Uuid::new_v4(),
            ip_address,
            closed: false,
            _app_state: app_state,
            _ws_state: Arc::new(init_state),
            channel: socket.split(),
            update_interval: interval(Duration::from_millis(update_interval_in_ms)),
            notify_receiver,
            exit: watch::channel(false),
        };

        subscriber.assert_is_healthy().await?;

        Ok(subscriber)
    }

    /// Perform the initial handshake with the client - ensure the channel is healthy
    async fn assert_is_healthy(&mut self) -> Result<(), WebSocketError> {
        let ping_status = self.channel.0.send(Message::Ping(vec![1, 2, 3])).await;
        if ping_status.is_err() {
            return Err(WebSocketError::ChannelInitError);
        }
        Ok(())
    }

    pub async fn listen<H, CommMsg>(&mut self, mut handler: H) -> Result<(), WebSocketError>
    where
        H: ChannelHandler<WsState, CommMsg>,
    {
        loop {
            tokio::select! {
                // Messages from the client
                maybe_client_msg = self.channel.1.next() => {
                    match maybe_client_msg {
                        Some(Ok(client_msg)) => {
                        tracing::info!("👤 [CLIENT MESSAGE]");
                            let client_msg = handler.decode_client_msg(self, client_msg).await;
                            if let Some(client_msg) = client_msg {
                                handler.handle_client_msg(self, client_msg).await?;
                            }
                        }
                        Some(Err(_)) => {
                            tracing::info!("Client disconnected or error occurred. Closing the channel.");
                            return Ok(());
                        },
                        None => {}
                    }
                },
                // Periodic updates
                _ = self.update_interval.tick() => {
                    tracing::info!("🕒 [PERIODIC INTERVAL]");
                    handler.periodic_interval(self).await?;
                },
                // Messages from the client to the server
                maybe_server_msg = self.notify_receiver.recv() => {
                    if let Some(server_msg) = maybe_server_msg {
                        tracing::info!("🥡 [SERVER MESSAGE]");
                        let server_msg = handler.decode_server_msg(self, server_msg).await;
                        if let Some(sever_msg) = server_msg {
                            handler.handle_server_msg(self, sever_msg).await?;
                        }
                    }
                },
                // Exit signal
                _ = self.exit.1.changed() => {
                    if *self.exit.1.borrow() {
                        tracing::info!("⛔ [CLOSING SIGNAL]");
                        return Ok(());
                    }
                },
            }
        }
    }
}
