use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToResponse, ToSchema};

use pragma_common::timestamp::{TimestampError, TimestampRangeError};
use pragma_common::types::pair::Pair;
use pragma_entities::VolatilityError;

use crate::AppState;
use crate::constants::PRAGMA_DECIMALS;
use crate::infra::repositories::entry_repository::{self, MedianEntry};
use crate::utils::PathExtractor;
use crate::utils::compute_volatility;

/// Volatility query
#[derive(Deserialize, IntoParams, Debug)]
pub struct VolatilityQuery {
    /// Initial timestamp, combined with `end`, it helps define the period over which the mean is computed
    start: u64,
    /// Final timestamp
    end: u64,
}

#[derive(Debug, Serialize, Deserialize, ToResponse, ToSchema)]
pub struct GetVolatilityResponse {
    pair_id: String,
    volatility: f64,
    decimals: u32,
}

#[utoipa::path(
        get,
        path = "/node/v1/volatility/{quote}/{base}",
        responses(
            (status = 200, description = "Get realized volatility successfuly", body = [GetVolatilityResponse])
        ),
        params(
            ("quote" = String, Path, description = "Quote Asset"),
            ("base" = String, Path, description = "Base Asset"),
            VolatilityQuery
        ),
    )]
#[tracing::instrument(skip(state))]
pub async fn get_volatility(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(volatility_query): Query<VolatilityQuery>,
) -> Result<Json<GetVolatilityResponse>, VolatilityError> {
    let pair = Pair::from(pair);

    if volatility_query.start > volatility_query.end {
        return Err(VolatilityError::InvalidTimestamp(
            TimestampError::RangeError(TimestampRangeError::StartAfterEnd),
        ));
    }

    // Fetch entries between start and end timestamps
    let entries = entry_repository::get_median_entries_1_min_between(
        &state.offchain_pool,
        pair.to_pair_id(),
        volatility_query.start,
        volatility_query.end,
    )
    .await?;

    if entries.is_empty() {
        return Err(VolatilityError::EntryNotFound(pair.to_pair_id()));
    }

    Ok(Json(adapt_entry_to_entry_response(pair.into(), &entries)))
}

fn adapt_entry_to_entry_response(
    pair_id: String,
    entries: &[MedianEntry],
) -> GetVolatilityResponse {
    let volatility = compute_volatility(entries);

    GetVolatilityResponse {
        pair_id,
        volatility,
        decimals: PRAGMA_DECIMALS,
    }
}
