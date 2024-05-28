use axum::extract::{Query, State};
use axum::Json;
use pragma_entities::EntryError;

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
) -> Result<Json<GetOnchainCheckpointsResponse>, EntryError> {
    tracing::info!("Received get onchain entry request for pair {:?}", pair);

    let pair_id: String = currency_pair_to_pair_id(&pair.0, &pair.1);
    let limit = if let Some(limit) = params.limit {
        if (limit == 0) || (limit > MAX_LIMIT) {
            // TODO(akhercha): not so great error kind
            return Err(EntryError::InvalidLimit(limit));
        }
        limit
    } else {
        DEFAULT_LIMIT
    };

    let decimals = get_decimals(&state.timescale_pool, &pair_id)
        .await
        .map_err(|db_error| db_error.to_entry_error(&pair_id))?;

    let checkpoints = get_checkpoints(
        &state.postgres_pool,
        params.network,
        pair_id.clone(),
        decimals,
        limit,
    )
    .await
    .map_err(|db_error| db_error.to_entry_error(&pair_id))?;

    Ok(Json(GetOnchainCheckpointsResponse(checkpoints)))
}
