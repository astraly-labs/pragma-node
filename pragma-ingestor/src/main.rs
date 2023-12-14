use tokio::sync::mpsc;
use tracing::info;

mod config;
mod consumer;
mod error;
mod subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    subscriber::init();
    info!(
        "kafka configuration : hostname={:?}, group_id={}, topic={}",
        config::CONFIG.kafka.brokers,
        config::CONFIG.kafka.group_id,
        config::CONFIG.kafka.topic
    );

    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    tokio::spawn(consumer::consume(tx));

    loop {
        while let Some(message) = rx.recv().await {
            info!("message received: {:?}", message);
        }
    }
}
