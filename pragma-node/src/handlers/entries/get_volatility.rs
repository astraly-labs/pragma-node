use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::handlers::entries::GetVolatilityResponse;
use crate::infra::repositories::entry_repository::{self, MedianEntry};
use crate::utils::PathExtractor;
use crate::AppState;
use pragma_entities::{error::InfraError, EntryError, VolatilityError};

use super::utils::{compute_volatility, currency_pair_to_pair_id};

/// Volatility query
#[derive(Deserialize, IntoParams)]
pub struct VolatilityQuery {
    /// Initial timestamp, combined with final_timestamp, it helps define the period over which the mean is computed
    start: u64,
    /// Final timestamp
    end: u64,
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
pub async fn get_volatility(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(volatility_query): Query<VolatilityQuery>,
) -> Result<Json<GetVolatilityResponse>, EntryError> {
    tracing::info!("Received get volatility request for pair {:?}", pair);
    // Construct pair id
    let pair_id = currency_pair_to_pair_id(&pair.0, &pair.1);

    if volatility_query.start > volatility_query.end {
        return Err(EntryError::VolatilityError(
            VolatilityError::InvalidTimestampsRange(volatility_query.start, volatility_query.end),
        ));
    }

    // Fetch entries between start and end timestamps
    let entries = entry_repository::get_entries_between(
        &state.offchain_pool,
        pair_id.clone(),
        volatility_query.start,
        volatility_query.end,
    )
    .await
    .map_err(|db_error| match db_error {
        InfraError::InternalServerError => EntryError::InternalServerError,
        InfraError::NotFound => EntryError::NotFound(pair_id.clone()),
        InfraError::InvalidTimeStamp => EntryError::InvalidTimestamp,
    })?;

    if entries.is_empty() {
        return Err(EntryError::UnknownPairId(pair_id));
    }

    let decimals = entry_repository::get_decimals(&state.offchain_pool, &pair_id)
        .await
        .map_err(|db_error| match db_error {
            InfraError::InternalServerError => EntryError::InternalServerError,
            InfraError::NotFound => EntryError::NotFound(pair_id.clone()),
            InfraError::InvalidTimeStamp => EntryError::InvalidTimestamp,
        })?;

    Ok(Json(adapt_entry_to_entry_response(
        pair_id, &entries, decimals,
    )))
}

fn adapt_entry_to_entry_response(
    pair_id: String,
    entries: &Vec<MedianEntry>,
    decimals: u32,
) -> GetVolatilityResponse {
    let volatility = compute_volatility(entries);

    GetVolatilityResponse {
        pair_id,
        volatility,
        decimals,
    }
}
