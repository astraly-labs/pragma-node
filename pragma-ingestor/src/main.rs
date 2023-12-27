use dotenvy::dotenv;
use pragma_entities::{adapt_infra_error, Entry, InfraError, NewEntry};
use tokio::sync::mpsc;
use tracing::{error, info};

mod config;
mod consumer;
mod error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().expect(".env file not found");
    pragma_common::tracing::init_tracing();
    info!(
        "kafka configuration : hostname={:?}, group_id={}, topic={}",
        config::CONFIG.brokers,
        config::CONFIG.group_id,
        config::CONFIG.topic
    );

    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    tokio::spawn(consumer::consume(tx));

    loop {
        // can be enhanced with struct like { type: "create_entry", data: Vec<T> }
        while let Some(payload) = rx.recv().await {
            if let Ok(entries) = serde_json::from_slice::<Vec<NewEntry>>(&payload) {
                info!("total of '{}' new entries available.", entries.len());
                if let Err(e) = insert_entries(entries).await {
                    error!("error while inserting entries : {:?}", e);
                }
            }
        }
    }
}

// TODO: move this to a service
pub async fn insert_entries(new_entries: Vec<NewEntry>) -> Result<(), InfraError> {
    let conn = pragma_entities::connection::init_pool("pragma-ingestor")
        .expect("cannot connect to database")
        .get()
        .await
        .expect("cannot get connection from pool");

    let entries = conn
        .interact(move |conn| Entry::create_many(conn, new_entries))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    for entry in &entries {
        info!(
            "new entry created {} - {}({}) - {}",
            entry.publisher, entry.pair_id, entry.price, entry.source
        );
    }

    Ok(())
}
