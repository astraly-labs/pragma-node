use axum::extract::{Query, State};
use axum::Json;
use pragma_entities::CheckpointError;

use crate::handlers::entries::utils::currency_pair_to_pair_id;
use crate::handlers::entries::{GetOnchainCheckpointsParams, GetOnchainCheckpointsResponse};
use crate::infra::repositories::entry_repository::get_decimals;
use crate::infra::repositories::onchain_repository::get_checkpoints;
use crate::utils::PathExtractor;
use crate::AppState;

pub const DEFAULT_LIMIT: u64 = 100;
pub const MAX_LIMIT: u64 = 1000;

#[utoipa::path(
    get,
    path = "/node/v1/onchain/checkpoints/{base}/{quote}",
    responses(
        (status = 200, description = "Get the onchain checkpoints for a pair", body = GetOnchainCheckpointsResponse)
    ),
    params(
        ("base" = String, Path, description = "Base Asset"),
        ("quote" = String, Path, description = "Quote Asset"),
        ("network" = Network, Query, description = "Network"),
        ("limit" = Option<u64>, Query, description = "Limit of response size")
    ),
)]
pub async fn get_onchain_checkpoints(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetOnchainCheckpointsParams>,
) -> Result<Json<GetOnchainCheckpointsResponse>, CheckpointError> {
    tracing::info!("Received get onchain entry request for pair {:?}", pair);

    let pair_id: String = currency_pair_to_pair_id(&pair.0, &pair.1);

    let limit = params.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(CheckpointError::InvalidLimit(limit));
    }

    let decimals = get_decimals(&state.timescale_pool, &pair_id)
        .await
        .map_err(CheckpointError::from)?;

    let checkpoints = get_checkpoints(
        &state.postgres_pool,
        params.network,
        pair_id.clone(),
        decimals,
        limit,
    )
    .await
    .map_err(CheckpointError::from)?;

    if checkpoints.is_empty() {
        return Err(CheckpointError::NotFound());
    }
    Ok(Json(GetOnchainCheckpointsResponse(checkpoints)))
}
