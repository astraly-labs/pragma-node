// https://docs.rs/redis/0.26.1/redis/#async

use axum::extract::{Query, State};
use axum::Json;
use pragma_common::types::merkle_tree::MerkleProof;
use pragma_common::types::Network;
use pragma_entities::models::merkle_feed_error::MerkleFeedError;
use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;
use utoipa::{IntoParams, ToSchema};

use crate::infra::redis;
use crate::utils::PathExtractor;
use crate::AppState;

#[derive(Default, Deserialize, IntoParams, ToSchema)]
pub struct GetMerkleProofQuery {
    pub network: Option<Network>,
    pub block_number: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetMerkleProofResponse(pub MerkleProof);

#[utoipa::path(
    get,
    path = "/node/v1/merkle_feeds/proof/{option_hash}",
    responses(
        (status = 200, description = "Get the merkle tree", body = [GetMerkleTreeResponse])
    ),
    params(
        ("option_hash" = String, Path, description = "Hexadecimal hash of the option"),
        GetMerkleProofQuery
    ),
)]
pub async fn get_merkle_feeds_proof(
    State(state): State<AppState>,
    PathExtractor(option_hex_hash): PathExtractor<String>,
    Query(params): Query<GetMerkleProofQuery>,
) -> Result<Json<GetMerkleProofResponse>, MerkleFeedError> {
    tracing::info!("Received get merkle tree request");
    if state.redis_client.is_none() {
        return Err(MerkleFeedError::RedisConnection);
    }

    let network = params.network.unwrap_or_default();
    let block_number = params.block_number;

    if !is_0x_prefixed_hex_string(&option_hex_hash) {
        return Err(MerkleFeedError::InvalidOptionHash(option_hex_hash.clone()));
    }

    let merkle_tree = redis::get_merkle_tree(
        state.redis_client.unwrap(),
        network,
        block_number,
        state.caches.merkle_feeds_tree().clone(),
    )
    .await
    .map_err(MerkleFeedError::from)?;

    let option_felt_hash = FieldElement::from_hex_be(&option_hex_hash)
        .map_err(|_| MerkleFeedError::InvalidOptionHash(option_hex_hash.clone()))?;

    let merkle_proof = merkle_tree.get_proof(&option_felt_hash);

    if merkle_proof.is_none() {
        return Err(MerkleFeedError::OptionNotFound(
            block_number,
            option_hex_hash,
        ));
    }

    Ok(Json(GetMerkleProofResponse(merkle_proof.unwrap())))
}

// Helper function to check if a string is a valid 0x-prefixed hexadecimal string
fn is_0x_prefixed_hex_string(s: &str) -> bool {
    s.starts_with("0x") && s[2..].chars().all(|c| c.is_ascii_hexdigit())
}
