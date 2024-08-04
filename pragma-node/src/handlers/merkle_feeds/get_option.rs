// https://docs.rs/redis/0.26.1/redis/#async

use axum::extract::{Query, State};
use axum::Json;
use pragma_common::types::options::OptionData;
use pragma_common::types::Network;
use pragma_entities::models::merkle_feed_error::MerkleFeedError;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::infra::redis;
use crate::utils::PathExtractor;
use crate::AppState;

#[derive(Default, Deserialize, IntoParams, ToSchema)]
pub struct GetOptionQuery {
    pub network: Option<Network>,
    pub block_number: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetOptionResponse {
    #[serde(flatten)]
    pub option_data: OptionData,
    pub hash: String,
}

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
pub async fn get_merkle_feeds_option(
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

    let network = params.network.unwrap_or_default();
    let block_number = params.block_number;

    let option_data = redis::get_option_from_redis(
        state.redis_client.unwrap(),
        network,
        block_number,
        instrument,
    )
    .await
    .map_err(MerkleFeedError::from)?;

    Ok(Json(GetOptionResponse {
        hash: option_data.hexadecimal_hash(),
        option_data,
    }))
}
