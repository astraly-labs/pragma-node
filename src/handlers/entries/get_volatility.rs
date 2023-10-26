use axum::extract::{Query, State};
use axum::Json;
use bigdecimal::ToPrimitive;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::domain::models::entry::{EntryError, VolatilityError};
use crate::handlers::entries::GetVolatilityResponse;
use crate::infra::errors::InfraError;
use crate::infra::repositories::entry_repository::{self, MedianEntry};
use crate::utils::PathExtractor;
use crate::AppState;

use super::utils::currency_pair_to_pair_id;

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
        path = "/v1/volatility/{quote}/{base}",
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
        &state.pool,
        pair_id.clone(),
        volatility_query.start,
        volatility_query.end,
    )
    .await
    .map_err(|db_error| match db_error {
        InfraError::InternalServerError => EntryError::InternalServerError,
        InfraError::NotFound => EntryError::NotFound(pair_id.clone()),
    })?;

    if entries.is_empty() {
        return Err(EntryError::UnknownPairId(pair_id));
    }

    let decimals = entry_repository::get_decimals(&state.pool, &pair_id)
        .await
        .map_err(|db_error| match db_error {
            InfraError::InternalServerError => EntryError::InternalServerError,
            InfraError::NotFound => EntryError::NotFound(pair_id.clone()),
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
    let mut values = Vec::new();
    for i in 1..entries.len() {
        if entries[i].median_price.to_f64().unwrap() > 0.0
            && entries[i - 1].median_price.to_f64().unwrap() > 0.0
            && (entries[i].time - entries[i - 1].time).num_seconds() > 0
        {
            let log_return = (entries[i].median_price.to_f64().unwrap()
                / entries[i - 1].median_price.to_f64().unwrap())
            .ln()
            .powi(2);

            let time = (entries[i].time - entries[i - 1].time)
                .num_seconds()
                .to_f64()
                .unwrap()
                / 31536000 as f64; // One year in seconds

            values.push((log_return, time));
        }
    }

    let variance: f64 = values.iter().map(|v| v.0 / v.1).sum::<f64>() / values.len() as f64;
    let volatility = variance.sqrt();

    GetVolatilityResponse {
        pair_id,
        volatility,
        decimals,
    }
}
