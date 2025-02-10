use axum::extract::State;
use axum::Json;
use chrono::NaiveDateTime;

use pragma_common::types::pair::Pair;
use pragma_entities::EntryError;

use crate::infra::repositories::entry_repository;
use crate::utils::PathExtractor;
use crate::AppState;

#[utoipa::path(
    get,
    path = "/node/v1/data/{base}/{quote}/future_expiries",
    responses(
        (status = 200, description = "Get available future expiries for a pair", body = [Vec<NaiveDateTime>])
    ),
    params(
        ("base" = String, Path, description = "Base Asset"),
        ("quote" = String, Path, description = "Quote Asset"),
    ),
)]
#[tracing::instrument(skip(state))]
pub async fn get_expiries(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
) -> Result<Json<Vec<NaiveDateTime>>, EntryError> {
    let pair = Pair::from(pair);

    let req_result = entry_repository::get_expiries_list(&state.offchain_pool, pair.to_pair_id())
        .await
        .map_err(|e| e.to_entry_error(&(pair.into())))?;

    Ok(Json(req_result))
}
