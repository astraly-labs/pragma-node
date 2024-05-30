use axum::extract::{Query, State};
use axum::Json;

use pragma_entities::EntryError;

use crate::handlers::entries::{GetOnchainOHLCParams, GetOnchainOHLCResponse};
use crate::infra::repositories::onchain_repository::get_ohlc;
use crate::utils::PathExtractor;
use crate::AppState;

use crate::handlers::entries::utils::currency_pair_to_pair_id;

pub const DEFAULT_LIMIT: u64 = 1000;
pub const MAX_LIMIT: u64 = 10000;

#[utoipa::path(
    get,
    path = "/node/v1/onchain/ohlc/{base}/{quote}",
    responses(
        (status = 200, description = "Get the onchain OHLC data for a pair", body = GetOnchainPublishersResponse)
    ),
    params(
        ("base" = String, Path, description = "Base Asset"),
        ("quote" = String, Path, description = "Quote Asset"),
        ("network" = Network, Query, description = "Network"),
        ("interval" = Interval, Query, description = "Interval of the OHLC data"),
        ("limit" = Option<u64>, Query, description = "Limit of response size")
    ),
)]
pub async fn get_onchain_ohlc(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetOnchainOHLCParams>,
) -> Result<Json<GetOnchainOHLCResponse>, EntryError> {
    let pair_id = currency_pair_to_pair_id(&pair.0, &pair.1);

    let limit = params.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(EntryError::BadRequest);
    }

    let ohlc_data = get_ohlc(
        &state.postgres_pool,
        params.network,
        pair_id.clone(),
        params.interval,
        limit,
    )
    .await
    .map_err(|db_err| db_err.to_entry_error(&pair_id))?;

    Ok(Json(GetOnchainOHLCResponse {
        pair_id,
        data: ohlc_data,
    }))
}
