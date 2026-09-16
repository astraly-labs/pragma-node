use axum::Json;
use axum::extract::{Query, State};
use pragma_common::{Pair, starknet::StarknetNetwork};
use pragma_entities::models::entries::timestamp::TimestampRange;
use pragma_entities::{EntryError, TimestampError};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::infra::repositories::onchain_repository::{
    get_onchain_decimals, observations::get_observations,
};
use crate::state::AppState;
use crate::utils::{PathExtractor, big_decimal_price_to_hex};

const MAX_OBSERVATIONS: usize = 20_000;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct GetObservationsParams {
    pub network: StarknetNetwork,
    pub timestamp: TimestampRange,
    pub publisher: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Observation {
    publisher: String,
    source: String,
    timestamp: i64,
    price: String,
    tx_hash: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ObservationsResponse {
    pair_id: String,
    decimals: u32,
    /// Zero means every observation; otherwise the latest observation per bucket and series.
    bucket_seconds: i64,
    truncated: bool,
    observations: Vec<Observation>,
}

fn bucket_seconds(start: i64, end: i64) -> Result<i64, EntryError> {
    if start < 0 || end - start > 7 * 86_400 {
        return Err(EntryError::InvalidTimestamp(TimestampError::Other(
            "Observation history is limited to seven days per request".into(),
        )));
    }
    Ok(if end - start > 86_400 { 1800 } else { 0 })
}

#[utoipa::path(
    get,
    path = "/node/v1/onchain/observations/{base}/{quote}",
    responses((status = 200, description = "Submitted observations, preserving publisher and source", body = ObservationsResponse)),
    params(("base" = String, Path), ("quote" = String, Path), GetObservationsParams),
)]
pub async fn get_onchain_observations(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetObservationsParams>,
) -> Result<Json<ObservationsResponse>, EntryError> {
    let pair = Pair::try_from(pair).map_err(|e| EntryError::InternalServerError(e.to_string()))?;
    let range = params
        .timestamp
        .assert_time_is_valid()
        .map_err(|e| EntryError::InvalidTimestamp(TimestampError::RangeError(e)))?;
    let bucket = bucket_seconds(*range.0.start(), *range.0.end())?;
    let mut rows = get_observations(
        &state.onchain_pool,
        params.network,
        &pair,
        range,
        params.publisher,
        params.source,
        bucket,
    )
    .await?;
    let truncated = rows.len() > MAX_OBSERVATIONS;
    rows.truncate(MAX_OBSERVATIONS);
    let decimals = get_onchain_decimals(
        state.caches.onchain_decimals(),
        &state.rpc_clients,
        params.network,
        &pair,
    )
    .await?;
    Ok(Json(ObservationsResponse {
        pair_id: pair.to_pair_id(),
        decimals,
        bucket_seconds: bucket,
        truncated,
        observations: rows
            .into_iter()
            .map(|row| Observation {
                publisher: row.publisher,
                source: row.source,
                timestamp: row.timestamp.and_utc().timestamp(),
                price: big_decimal_price_to_hex(&row.price),
                tx_hash: row.tx_hash,
            })
            .collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_intraday_observations_and_bounds_long_queries() {
        assert_eq!(bucket_seconds(100, 86_500).unwrap(), 0);
        assert_eq!(bucket_seconds(100, 604_900).unwrap(), 1800);
        assert!(bucket_seconds(100, 604_901).is_err());
        assert!(bucket_seconds(-1, 3600).is_err());
    }
}
