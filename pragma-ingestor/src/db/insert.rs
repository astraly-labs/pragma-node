use deadpool_diesel::postgres::Pool;
use pragma_entities::{
    Entry, FundingRate, FutureEntry, InfraError, NewEntry, NewFundingRate, NewFutureEntry,
    NewOpenInterest,
};
use tracing::debug;

#[tracing::instrument(skip_all, fields(num_entries = new_entries.len()))]
pub(crate) async fn insert_spot_entries(
    pool: &Pool,
    new_entries: Vec<NewEntry>,
) -> Result<(), InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    conn.interact(move |conn| Entry::create_many(conn, new_entries))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(num_entries = new_entries.len()))]
pub(crate) async fn insert_future_entries(
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

    conn.interact(move |conn| FutureEntry::create_many(conn, new_entries))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;
    Ok(())
}

#[tracing::instrument(skip_all, fields(num_entries = new_entries.len()))]
pub(crate) async fn insert_funding_rate_entries(
    pool: &Pool,
    new_entries: Vec<NewFundingRate>,
) -> Result<(), InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let entries = conn
        .interact(move |conn| FundingRate::create_many(conn, new_entries))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    for entry in &entries {
        debug!(
            "new funding rate entry created {} - {}({}) - {}",
            entry.source, entry.pair, entry.annualized_rate, entry.timestamp
        );
    }

    Ok(())
}

#[tracing::instrument(skip_all, fields(num_entries = new_entries.len()))]
pub(crate) async fn insert_open_interest_entries(
    pool: &Pool,
    new_entries: Vec<NewOpenInterest>,
) -> Result<(), InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let entries = conn
        .interact(move |conn| OpenInterest::create_many(conn, new_entries))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    for entry in &entries {
        debug!(
            "new open interest entry created {} - {}({}) - {}",
            entry.source, entry.pair, entry.open_interest_value, entry.timestamp
        );
    }

    Ok(())
}
