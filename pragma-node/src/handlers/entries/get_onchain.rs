use axum::Json;

use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::utils::PathExtractor;
use crate::AppState;
use pragma_entities::EntryError;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetOnchainResponse {
    todo: String,
}

/// Volatility query
#[derive(Deserialize, IntoParams)]
pub struct GetOnchainQuery {
    _todo: u64,
}
pub async fn get_onchain(
    State(_state): State<AppState>,
    PathExtractor(_pair): PathExtractor<(String, String)>,
    Query(_onchain_query): Query<GetOnchainQuery>,
) -> Result<Json<GetOnchainResponse>, EntryError> {
    let res = GetOnchainResponse {
        todo: "todo".to_string(),
    };
    Ok(Json(res))
}
