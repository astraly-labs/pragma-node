use axum::Json;

use axum::extract::{Query, State};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::utils::PathExtractor;
use crate::AppState;
use pragma_entities::EntryError;

use crate::handlers::entries::GetOnchainResponse;

/// Onchain query
#[derive(Deserialize, IntoParams)]
pub struct OnchainQuery {
    _todo: u64,
}

#[utoipa::path(
    get,
    path = "/node/v1/onchain/{todo}",
    responses(
        (status = 200, description = "Get on chain data", body = [GetOnchainResponse])
    ),
    params(
        ("todo" = String, Path, description = "TODO - not done yet"),
        OnchainQuery
    ),
)]
pub async fn get_onchain(
    State(_state): State<AppState>,
    PathExtractor(_pair): PathExtractor<(String, String)>,
    Query(_onchain_query): Query<OnchainQuery>,
) -> Result<Json<GetOnchainResponse>, EntryError> {
    let res = GetOnchainResponse {
        todo: "todo".to_string(),
    };
    Ok(Json(res))
}
