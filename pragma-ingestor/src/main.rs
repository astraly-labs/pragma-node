use dotenvy::dotenv;
use pragma_entities::connection::ENV_TS_DATABASE_URL;
use pragma_entities::{adapt_infra_error, Entry, InfraError, NewEntry, NewPerpEntry, PerpEntry};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{error, info};
mod config;
mod consumer;
mod error;

#[derive(Deserialize)]
#[serde(untagged)]
enum EntriesRequest {
    NewEntries(Vec<NewEntry>),
    NewPerpEntries(Vec<NewPerpEntry>),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenv(); // .env file is not present in prod
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
        while let Some(payload) = rx.recv().await {
            if let Err(e) = process_payload(payload).await {
                error!("error while processing payload: {:?}", e);
            }
        }
    }
}

async fn process_payload(payload: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    match serde_json::from_slice::<EntriesRequest>(&payload) {
        Ok(EntriesRequest::NewEntries(entries)) => {
            info!("total of '{}' new entries available.", entries.len());
            if let Err(e) = insert_entries(entries).await {
                error!("error while inserting entries : {:?}", e);
            }
        }
        Ok(EntriesRequest::NewPerpEntries(perp_entries)) => {
            info!(
                "total of '{}' new perp entries available.",
                perp_entries.len()
            );
            if let Err(e) = insert_perp_entries(perp_entries).await {
                error!("error while inserting perp entries : {:?}", e);
            }
        }
        Err(e) => {
            error!("Failed to deserialize payload: {:?}", e);
        }
    }
    Ok(())
}

// TODO: move this to a service
pub async fn insert_entries(new_entries: Vec<NewEntry>) -> Result<(), InfraError> {
    let conn = pragma_entities::connection::init_pool("pragma-ingestor", ENV_TS_DATABASE_URL)
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

// TODO: move this to a service
// TODO: refactor with function above to avoid duplication
pub async fn insert_perp_entries(new_perp_entries: Vec<NewPerpEntry>) -> Result<(), InfraError> {
    let conn = pragma_entities::connection::init_pool("pragma-ingestor", ENV_TS_DATABASE_URL)
        .expect("cannot connect to database")
        .get()
        .await
        .expect("cannot get connection from pool");

    let entries = conn
        .interact(move |conn| PerpEntry::create_many(conn, new_perp_entries))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    for entry in &entries {
        info!(
            "new perp entry created {} - {}({}) - {}",
            entry.publisher, entry.pair_id, entry.price, entry.source
        );
    }

    Ok(())
}
