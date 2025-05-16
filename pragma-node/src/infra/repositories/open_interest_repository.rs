use chrono::{DateTime, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::RunQueryDsl as _;
use pragma_common::Pair;
use pragma_entities::{
    InfraError, TimestampError, models::entries::timestamp::TimestampRange,
    models::open_interest::OpenInterest,
};
use serde::Serialize;
use diesel::sql_types::{Timestamp, VarChar};

pub async fn get_at_timestamp(
    pool: &Pool,
    pair: Pair,
    source: String,
    timestamp: Option<i64>,
) -> Result<Option<OpenInterest>, InfraError> {
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

    let open_interest = conn
        .interact(move |conn| {
            if let Some(ts) = timestamp {
                OpenInterest::get_at(conn, &pair, &source, ts)
            } else {
                OpenInterest::get_latest(conn, &pair, &source)
            }
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(open_interest)
}

pub async fn get_history_in_range(
    pool: &Pool,
    pair: Pair,
    source: String,
    range: TimestampRange,
) -> Result<Vec<OpenInterest>, InfraError> {
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

    let open_interests = conn
        .interact(move |conn| OpenInterest::get_in_range(conn, &pair, &source, start, end))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(open_interests)
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct InstrumentInfo {
    pub pair: String,
    pub source: String,
    pub first_timestamp_ms: u64,
    pub last_timestamp_ms: u64,
}

#[derive(diesel::QueryableByName)]
struct InstrumentDTO {
    #[diesel(sql_type = VarChar)]
    pair: String,
    #[diesel(sql_type = VarChar)]
    source: String,
    #[diesel(sql_type = Timestamp)]
    first_ts: chrono::NaiveDateTime,
    #[diesel(sql_type = Timestamp)]
    last_ts: chrono::NaiveDateTime,
}

pub async fn get_supported_instruments(pool: &Pool) -> Result<Vec<InstrumentInfo>, InfraError> {
    let sql = r#"
        SELECT
            pair,
            source,
            MIN(timestamp) AS first_ts,
            MAX(timestamp) AS last_ts
        FROM open_interest
        GROUP BY pair, source
        ORDER BY pair, source;
    "#;

    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let rows: Vec<InstrumentDTO> = conn
        .interact(move |c| diesel::sql_query(sql).load::<InstrumentDTO>(c))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(rows
        .into_iter()
        .map(|r| InstrumentInfo {
            pair: r.pair,
            source: r.source,
            first_timestamp_ms: r.first_ts.and_utc().timestamp_millis() as u64,
            last_timestamp_ms: r.last_ts.and_utc().timestamp_millis() as u64,
        })
        .collect())
} 