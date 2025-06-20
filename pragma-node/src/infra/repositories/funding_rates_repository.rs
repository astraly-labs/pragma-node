use chrono::{DateTime, Utc};
use deadpool_diesel::postgres::Pool;

use diesel::{
    RunQueryDsl as _,
    sql_types::{Timestamp, VarChar},
};
use pragma_common::Pair;
use pragma_entities::{
    FundingRate, InfraError, PaginationParams, TimestampError,
    models::entries::timestamp::TimestampRange,
};
use serde::Serialize;

use crate::handlers::funding_rates::get_historical_funding_rates::Frequency;

pub async fn get_at_timestamp(
    pool: &Pool,
    pair: Pair,
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
                FundingRate::get_at(conn, &pair, &source, ts)
            } else {
                FundingRate::get_latest(conn, &pair, &source)
            }
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(funding_rate)
}

pub async fn get_history_in_range(
    pool: &Pool,
    pair: Pair,
    source: String,
    range: TimestampRange,
    frequency: Frequency,
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
        .interact(move |conn| match frequency {
            Frequency::All => FundingRate::get_in_range(conn, &pair, &source, start, end),
            Frequency::Minute => FundingRate::get_in_range_aggregated(
                conn,
                &pair,
                &source,
                start,
                end,
                "funding_rates_1_min",
            ),
            Frequency::Hour => FundingRate::get_in_range_aggregated(
                conn,
                &pair,
                &source,
                start,
                end,
                "funding_rates_1_hour",
            ),
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(funding_rates)
}

pub async fn get_history_in_range_paginated(
    pool: &Pool,
    pair: Pair,
    source: String,
    range: TimestampRange,
    frequency: Frequency,
    pagination: PaginationParams,
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
        .interact(move |conn| match frequency {
            Frequency::All => {
                FundingRate::get_in_range_paginated(conn, &pair, &source, start, end, &pagination)
            }
            Frequency::Minute => FundingRate::get_in_range_aggregated_paginated(
                conn,
                &pair,
                &source,
                start,
                end,
                "funding_rates_1_min",
                &pagination,
            ),
            Frequency::Hour => FundingRate::get_in_range_aggregated_paginated(
                conn,
                &pair,
                &source,
                start,
                end,
                "funding_rates_1_hour",
                &pagination,
            ),
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(funding_rates)
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct InstrumentInfo {
    pub pair: String,
    pub source: String,
    /// premier timestamp disponible (ms Unix)
    pub first_timestamp_ms: u64,
    /// dernier timestamp disponible (ms Unix)
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
    let sql = r"
        SELECT
            pair,
            source,
            first_ts,
            last_ts
        FROM funding_rates_instruments_summary
        ORDER BY pair, source;
    ";

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
