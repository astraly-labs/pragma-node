use deadpool_diesel::postgres::Pool;
use dotenvy::dotenv;
use pragma_entities::connection::ENV_TS_DATABASE_URL;
use pragma_entities::{
    adapt_infra_error, Entry, FutureEntry, InfraError, NewEntry, NewFutureEntry, NewPerpEntry,
    PerpEntry,
};
use tokio::sync::mpsc;
use tracing::{error, info};
mod config;
mod consumer;
mod error;

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

    let pool = pragma_entities::connection::init_pool("pragma-ingestor", ENV_TS_DATABASE_URL)
        .expect("cannot connect to database");

    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    tokio::spawn(consumer::consume(tx));
    loop {
        while let Some(payload) = rx.recv().await {
            if let Err(e) = process_payload(&pool, payload).await {
                error!("error while processing payload: {:?}", e);
            }
        }
    }
}

async fn process_payload(pool: &Pool, payload: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let decoded_payload = String::from_utf8_lossy(&payload);
    let is_future_entries = decoded_payload.contains("expiration_timestamp");
    if !is_future_entries {
        match serde_json::from_slice::<Vec<NewEntry>>(&payload) {
            Ok(entries) => {
                info!("[SPOT] total of '{}' new entries available.", entries.len());
                if let Err(e) = insert_entries(pool, entries).await {
                    error!("error while inserting entries : {:?}", e);
                }
            }
            Err(e) => {
                error!("Failed to deserialize payload: {:?}", e);
            }
        }
    } else {
        match serde_json::from_slice::<Vec<NewFutureEntry>>(&payload) {
            Ok(future_entries) => {
                info!(
                    "[FUTURE/PERP] total of '{}' new entries available.",
                    future_entries.len()
                );
                if let Err(e) = insert_future_entries(pool, future_entries).await {
                    error!("error while inserting future entries : {:?}", e);
                }
            }
            Err(e) => {
                error!("Failed to deserialize payload: {:?}", e);
            }
        }
    }
    Ok(())
}

// TODO: move this to a service
pub async fn insert_entries(pool: &Pool, new_entries: Vec<NewEntry>) -> Result<(), InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
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

/// Insert new future entries into the database.
/// Future entries means future & perpetual entries.
/// If the expiration timestamp is not provided, the entry is considered as a perpetual entry.
/// It will be then inserted into the perp_entries table.
// TODO: move this to a service
pub async fn insert_future_entries(
    pool: &Pool,
    new_entries: Vec<NewFutureEntry>,
) -> Result<(), InfraError> {
    // TODO(akhercha): creating a connection pool for each request is not efficient
    // filter out in two groups: perp & future
    let new_perp_entries = new_entries
        .iter()
        .filter(|entry| entry.expiration_timestamp.and_utc().timestamp() == 0)
        .map(|entry| NewPerpEntry::from(entry.clone()))
        .collect::<Vec<NewPerpEntry>>();

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    if !new_perp_entries.is_empty() {
        info!(
            "[PERP] total of '{}' new entries available.",
            new_perp_entries.len()
        );
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
    }

    let new_future_entries = new_entries
        .iter()
        .filter(|entry| entry.expiration_timestamp.and_utc().timestamp() > 0)
        .cloned()
        .collect::<Vec<NewFutureEntry>>();

    if !new_future_entries.is_empty() {
        info!(
            "[FUTURE] total of '{}' new entries available.",
            new_future_entries.len()
        );
        let entries = conn
            .interact(move |conn| FutureEntry::create_many(conn, new_future_entries))
            .await
            .map_err(adapt_infra_error)?
            .map_err(adapt_infra_error)?;
        for entry in &entries {
            info!(
                "new future entry created {} - {}({}) - {}",
                entry.publisher, entry.pair_id, entry.price, entry.source
            );
        }
    }

    Ok(())
}
