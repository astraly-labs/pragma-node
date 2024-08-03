// https://docs.rs/redis/0.26.1/redis/#async

use axum::extract::{Query, State};
use axum::Json;
use pragma_common::types::Network;
use pragma_entities::models::merkle_feed_error::MerkleFeedError;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::AppState;

#[derive(Default, Deserialize, IntoParams, ToSchema)]
pub struct GetMerkleTreeQuery {
    pub network: Option<Network>,
    pub block_number: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetMerkleTreeResponse {}

#[utoipa::path(
    get,
    path = "/node/v1/merkle_feeds/tree",
    responses(
        (status = 200, description = "Get the merkle tree", body = [GetMerkleTreeResponse])
    ),
    params(
        GetMerkleTreeQuery
    ),
)]
pub async fn get_merkle_feeds_tree(
    State(state): State<AppState>,
    Query(params): Query<GetMerkleTreeQuery>,
) -> Result<Json<GetMerkleTreeResponse>, MerkleFeedError> {
    tracing::info!("Received get merkle tree request");
    if state.redis_client.is_none() {
        return Err(MerkleFeedError::RedisConnection);
    }

    let _network = params.network.unwrap_or_default();
    let _block_number = params.block_number;

    Ok(Json(GetMerkleTreeResponse {}))
}
