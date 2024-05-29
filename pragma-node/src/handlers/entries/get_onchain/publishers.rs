use axum::extract::{Query, State};
use axum::Json;

use pragma_entities::EntryError;

use crate::handlers::entries::{GetOnchainPublishersParams, GetOnchainPublishersResponse};
use crate::infra::repositories::onchain_repository::{get_publishers, get_publishers_updates};
use crate::AppState;

#[utoipa::path(
    get,
    path = "/node/v1/onchain/publishers",
    responses(
        (status = 200, description = "Get the onchain publishers", body = GetOnchainPublishersResponse)
    ),
    params(
        ("network" = Network, Query, description = "Network"),
    ),
)]
pub async fn get_onchain_publishers(
    State(state): State<AppState>,
    Query(params): Query<GetOnchainPublishersParams>,
) -> Result<Json<GetOnchainPublishersResponse>, EntryError> {
    let publishers = get_publishers(&state.timescale_pool)
        .await
        .map_err(|e| e.to_entry_error(&"".to_string()))?;

    let updates = get_publishers_updates(&state.postgres_pool, publishers, params.network)
        .await
        .map_err(|e| e.to_entry_error(&"".to_string()))?;

    Ok(Json(GetOnchainPublishersResponse(updates)))
}
