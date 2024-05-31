use axum::extract::{Query, State};
use axum::Json;

use pragma_entities::EntryError;

use crate::handlers::entries::{GetOnchainPublishersParams, GetOnchainPublishersResponse};
use crate::infra::repositories::entry_repository::get_all_currencies_decimals;
use crate::infra::repositories::onchain_repository::{
    get_publishers, get_publishers_with_components,
};
use crate::AppState;

#[utoipa::path(
    get,
    path = "/node/v1/onchain/publishers",
    responses(
        (status = 200, description = "Get the onchain publishers", body = GetOnchainPublishersResponse)
    ),
    params(
        ("network" = Network, Query, description = "Network"),
        ("data_type" = DataType, Query, description = "Data type"),
    ),
)]
pub async fn get_onchain_publishers(
    State(state): State<AppState>,
    Query(params): Query<GetOnchainPublishersParams>,
) -> Result<Json<GetOnchainPublishersResponse>, EntryError> {
    let publishers = get_publishers(&state.postgres_pool, params.network)
        .await
        .map_err(EntryError::from)?;

    let currencies_decimals = get_all_currencies_decimals(&state.timescale_pool)
        .await
        .map_err(EntryError::from)?;

    let publishers_with_components = get_publishers_with_components(
        &state.postgres_pool,
        params.network,
        params.data_type,
        currencies_decimals,
        publishers,
    )
    .await
    .map_err(EntryError::from)?;

    Ok(Json(GetOnchainPublishersResponse(
        publishers_with_components,
    )))
}
