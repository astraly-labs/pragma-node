use deadpool_diesel::postgres::Pool;
use pragma_entities::{Entry, FutureEntry, InfraError, NewEntry, NewFutureEntry};
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::debug;

#[tracing::instrument(skip(pool, rx))]
pub async fn process_spot_entries(pool: Pool, mut rx: mpsc::Receiver<NewEntry>) {
    const BUFFER_CAPACITY: usize = 100;
    const FLUSH_TIMEOUT: Duration = Duration::from_millis(50);

    let mut buffer = Vec::with_capacity(BUFFER_CAPACITY);
    let mut flush_interval = interval(FLUSH_TIMEOUT);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                buffer.push(entry);

                if buffer.len() >= BUFFER_CAPACITY {
                    if let Err(e) = insert_spot_entries(&pool, std::mem::take(&mut buffer)).await {
                        tracing::error!("❌ Failed to insert spot entries: {}", e);
                    }
                    buffer = Vec::with_capacity(BUFFER_CAPACITY);
                }
            }
            _ = flush_interval.tick() => {
                if !buffer.is_empty() {
                    if let Err(e) = insert_spot_entries(&pool, std::mem::take(&mut buffer)).await {
                        tracing::error!("❌ Failed to flush spot entries: {}", e);
                    }
                    buffer = Vec::with_capacity(BUFFER_CAPACITY);
                }
            }
            else => {
                // Channel closed, flush remaining entries
                if !buffer.is_empty() {
                    if let Err(e) = insert_spot_entries(&pool, buffer).await {
                        tracing::error!("❌ Failed to flush final spot entries: {}", e);
                    }
                }
                break;
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx))]
pub async fn process_future_entries(pool: Pool, mut rx: mpsc::Receiver<NewFutureEntry>) {
    const BUFFER_CAPACITY: usize = 100;
    const FLUSH_TIMEOUT: Duration = Duration::from_secs(30);

    let mut buffer = Vec::with_capacity(BUFFER_CAPACITY);
    let mut flush_interval = interval(FLUSH_TIMEOUT);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                buffer.push(entry);

                if buffer.len() >= BUFFER_CAPACITY {
                    if let Err(e) = insert_future_entries(&pool, std::mem::take(&mut buffer)).await {
                        tracing::error!("❌ Failed to insert future entries: {}", e);
                    }
                    buffer = Vec::with_capacity(BUFFER_CAPACITY);
                }
            }
            _ = flush_interval.tick() => {
                if !buffer.is_empty() {
                    if let Err(e) = insert_future_entries(&pool, std::mem::take(&mut buffer)).await {
                        tracing::error!("❌ Failed to flush future entries: {}", e);
                    }
                    buffer = Vec::with_capacity(BUFFER_CAPACITY);
                }
            }
            else => {
                // Channel closed, flush remaining entries
                if !buffer.is_empty() {
                    if let Err(e) = insert_future_entries(&pool, buffer).await {
                        tracing::error!("❌ Failed to flush final future entries: {}", e);
                    }
                }
                break;
            }
        }
    }
}

#[tracing::instrument(skip(pool))]
async fn insert_spot_entries(pool: &Pool, new_entries: Vec<NewEntry>) -> Result<(), InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let entries = conn
        .interact(move |conn| Entry::create_many(conn, new_entries))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    for entry in &entries {
        debug!(
            "new spot entry created {} - {}({}) - {}",
            entry.publisher, entry.pair_id, entry.price, entry.source
        );
    }

    Ok(())
}

#[tracing::instrument(skip(pool))]
async fn insert_future_entries(
    pool: &Pool,
    new_entries: Vec<NewFutureEntry>,
) -> Result<(), InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let new_entries = new_entries
        .into_iter()
        .map(|mut entry| {
            if let Some(expiration_timestamp) = entry.expiration_timestamp {
                if expiration_timestamp.and_utc().timestamp() == 0 {
                    entry.expiration_timestamp = None;
                }
            }
            entry
        })
        .collect::<Vec<_>>();

    debug!(
        "[PERP] {} new entries available",
        new_entries
            .iter()
            .filter(|entry| entry.expiration_timestamp.is_none())
            .count()
    );

    let entries = conn
        .interact(move |conn| FutureEntry::create_many(conn, new_entries))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;
    for entry in &entries {
        debug!(
            "new perp entry created {} - {}({}) - {}",
            entry.publisher, entry.pair_id, entry.price, entry.source
        );
    }
    Ok(())
}
