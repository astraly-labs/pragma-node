use dotenvy::dotenv;
use tokio::sync::mpsc;
use tracing::info;
mod config;
mod consumer;
mod error;
mod subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().expect(".env file not found");
    subscriber::init();
    info!(
        "kafka configuration : hostname={:?}, group_id={}, topic={}",
        config::CONFIG.brokers,
        config::CONFIG.group_id,
        config::CONFIG.topic
    );

    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    tokio::spawn(consumer::consume(tx));

    loop {
        while let Some(message) = rx.recv().await {
            info!("message received: {:?}", message);
        }
    }
}
