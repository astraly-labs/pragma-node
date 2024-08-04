use std::sync::Arc;

use redis::JsonAsyncCommands;

use pragma_common::types::{
    merkle_tree::{MerkleTree, MerkleTreeError},
    options::OptionData,
    Network,
};
use pragma_entities::InfraError;
use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;

pub async fn get_option_from_redis(
    redis_client: Arc<redis::Client>,
    network: Network,
    block_number: u64,
    instrument_name: String,
) -> Result<OptionData, InfraError> {
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| InfraError::InternalServerError)?;

    let instrument_key = format!("{}/{}/options/{}", network, block_number, instrument_name);

    let result: String = conn
        .json_get(instrument_key, "$")
        .await
        .map_err(|_| InfraError::NotFound)?;

    // Redis [json_get] method returns a list of objects
    let mut option_response: Vec<OptionData> = serde_json::from_str(&result).map_err(|e| {
        tracing::error!("Error while deserialzing: {e}");
        InfraError::InternalServerError
    })?;

    if option_response.len() != 1 {
        return Err(InfraError::NotFound);
    }

    // Safe to unwrap, see condition above
    Ok(option_response.pop().unwrap())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawMerkleTree {
    leaves: Vec<String>,
    root_hash: String,
    levels: Vec<Vec<String>>,
    hash_method: String,
}

impl TryFrom<RawMerkleTree> for MerkleTree {
    type Error = MerkleTreeError;

    fn try_from(serialized_tree: RawMerkleTree) -> Result<Self, Self::Error> {
        let leaves: Vec<FieldElement> = serialized_tree
            .leaves
            .into_iter()
            .map(|leaf| FieldElement::from_hex_be(&leaf))
            .collect::<Result<Vec<FieldElement>, _>>()
            .map_err(|e| MerkleTreeError::BuildFailed(e.to_string()))?;

        let merkle_tree = MerkleTree::new(leaves)?;

        let expected_hash = FieldElement::from_hex_be(&serialized_tree.root_hash)
            .map_err(|e| MerkleTreeError::BuildFailed(e.to_string()))?;

        if merkle_tree.root_hash != expected_hash {
            return Err(MerkleTreeError::BuildFailed(format!(
                "Invalid built hash, found {}, expected {}.",
                merkle_tree.root_hash, expected_hash
            )));
        }

        Ok(merkle_tree)
    }
}

// TODO: This function deserves a Cache, so we don't rebuild
//       the same merkle tree everytime for the same block.
pub async fn get_merkle_tree_from_redis(
    redis_client: Arc<redis::Client>,
    network: Network,
    block_number: u64,
) -> Result<MerkleTree, InfraError> {
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| InfraError::InternalServerError)?;

    let instrument_key = format!("{}/{}/merkle_tree", network, block_number);
    tracing::info!("{}", instrument_key);

    let result: String = conn
        .json_get(instrument_key, "$")
        .await
        .map_err(|_| InfraError::NotFound)?;

    // Redis [json_get] method returns a list of objects
    let mut tree_response: Vec<RawMerkleTree> = serde_json::from_str(&result).map_err(|e| {
        tracing::error!("Error while deserialzing: {e}");
        InfraError::InternalServerError
    })?;

    if tree_response.len() != 1 {
        return Err(InfraError::NotFound);
    }

    let merkle_tree = MerkleTree::try_from(tree_response.pop().unwrap())
        .map_err(|_| InfraError::InternalServerError)?;
    Ok(merkle_tree)
}
