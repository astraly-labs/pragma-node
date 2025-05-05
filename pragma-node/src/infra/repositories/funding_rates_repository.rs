use chrono::{DateTime, Utc};
use deadpool_diesel::postgres::Pool;
use pragma_entities::{
    FundingRate, InfraError, TimestampError, models::entries::timestamp::TimestampRange,
};

pub async fn get_at_timestamp(
    pool: &Pool,
    pair: String,
    source: String,
    timestamp: Option<i64>,
) -> Result<Option<FundingRate>, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let timestamp = match timestamp {
        Some(ts) => Some(
            DateTime::<Utc>::from_timestamp(ts, 0)
                .ok_or_else(|| {
                    InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorI64(ts))
                })?
                .naive_utc(),
        ),
        None => None,
    };

    let funding_rate = conn
        .interact(move |conn| {
            if let Some(ts) = timestamp {
                FundingRate::get_at_or_before(conn, pair, source, ts)
            } else {
                FundingRate::get_latest(conn, pair, source)
            }
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(funding_rate)
}

pub async fn get_history_in_range(
    pool: &Pool,
    pair: String,
    source: String,
    range: TimestampRange,
) -> Result<Vec<FundingRate>, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let start = DateTime::<Utc>::from_timestamp(*range.0.start(), 0)
        .ok_or_else(|| {
            InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorI64(*range.0.start()))
        })?
        .naive_utc();

    let end = DateTime::<Utc>::from_timestamp(*range.0.end(), 0)
        .ok_or_else(|| {
            InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorI64(*range.0.end()))
        })?
        .naive_utc();

    let funding_rates = conn
        .interact(move |conn| FundingRate::get_in_range(conn, pair, source, start, end))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(funding_rates)
}
