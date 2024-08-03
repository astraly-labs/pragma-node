// https://docs.rs/redis/0.26.1/redis/#async

use axum::extract::{Query, State};
use axum::Json;
use pragma_common::types::merkle_tree::MerkleTree;
use pragma_common::types::Network;
use pragma_entities::models::merkle_feed_error::MerkleFeedError;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::infra::repositories::merkle_feeds_repository;
use crate::AppState;

#[derive(Default, Deserialize, IntoParams, ToSchema)]
pub struct GetMerkleTreeQuery {
    pub network: Option<Network>,
    pub block_number: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetMerkleTreeResponse(pub MerkleTree);

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

    let network = params.network.unwrap_or_default();
    let block_number = params.block_number;

    let merkle_tree = merkle_feeds_repository::get_merkle_tree_from_redis(
        state.redis_client.unwrap(),
        network,
        block_number,
    )
    .await
    .map_err(MerkleFeedError::from)?;

    Ok(Json(GetMerkleTreeResponse(merkle_tree)))
}
