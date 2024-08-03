// https://docs.rs/redis/0.26.1/redis/#async

use axum::extract::{Query, State};
use axum::Json;
use pragma_common::types::Network;
use pragma_entities::models::merkle_feed_error::MerkleFeedError;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::utils::PathExtractor;
use crate::AppState;

#[derive(Default, Deserialize, IntoParams, ToSchema)]
pub struct GetOptionQuery {
    pub network: Option<Network>,
    pub block_number: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetOptionResponse {}

#[utoipa::path(
    get,
    path = "/node/v1/merkle_feeds/options/{instrument}",
    responses(
        (status = 200, description = "Get the option", body = [GetOptionResponse])
    ),
    params(
        ("instrument" = String, Path, description = "Name of the instrument"),
        GetOptionQuery
    ),
)]
pub async fn get_option(
    State(state): State<AppState>,
    PathExtractor(instrument): PathExtractor<String>,
    Query(params): Query<GetOptionQuery>,
) -> Result<Json<GetOptionResponse>, MerkleFeedError> {
    tracing::info!(
        "Received get option request for instrument {:?}",
        instrument
    );
    if state.redis_client.is_none() {
        return Err(MerkleFeedError::RedisConnection);
    }

    let _network = params.network.unwrap_or_default();
    let _block_number = params.block_number;

    Ok(Json(GetOptionResponse {}))
}
