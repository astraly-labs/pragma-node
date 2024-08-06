use std::sync::Arc;

use moka::future::Cache;
use redis::{AsyncCommands, JsonAsyncCommands};
use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;

use pragma_common::types::{
    block_id::{BlockId, BlockTag},
    merkle_tree::{MerkleTree, MerkleTreeError},
    options::OptionData,
    Network,
};
use pragma_entities::error::RedisError;

pub async fn get_option_data(
    redis_client: Arc<redis::Client>,
    network: Network,
    block_id: BlockId,
    instrument_name: String,
) -> Result<OptionData, RedisError> {
    let block_number = get_block_number_from_id(&redis_client, &network, &block_id).await?;

    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| RedisError::Connection)?;

    let instrument_key = format!("{}/{}/options/{}", network, block_number, instrument_name);

    let result: String = conn
        .json_get(instrument_key, "$")
        .await
        .map_err(|_| RedisError::OptionNotFound(block_number, instrument_name.clone()))?;

    // Redis [json_get] method returns a list of objects
    let mut option_response: Vec<OptionData> = serde_json::from_str(&result).map_err(|e| {
        tracing::error!("Error while deserialzing: {e}");
        RedisError::InternalServerError
    })?;

    if option_response.len() != 1 {
        return Err(RedisError::OptionNotFound(block_number, instrument_name));
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

pub async fn get_merkle_tree(
    redis_client: Arc<redis::Client>,
    network: Network,
    block_id: BlockId,
    merkle_tree_cache: Cache<u64, MerkleTree>,
) -> Result<MerkleTree, RedisError> {
    let block_number = get_block_number_from_id(&redis_client, &network, &block_id).await?;

    // Try to retrieve the latest available cached value, and return it if it exists
    let maybe_cached_value = merkle_tree_cache.get(&block_number).await;
    if let Some(cached_value) = maybe_cached_value {
        tracing::debug!("Found a cached value for merkle tree at block {block_number} - using it.");
        return Ok(cached_value);
    }
    tracing::debug!(
        "No cache found for merkle tree at block {block_number}, fetching it from Redis."
    );

    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| RedisError::Connection)?;

    let instrument_key = format!("{}/{}/merkle_tree", network, block_number);

    let result: String = conn
        .json_get(instrument_key, "$")
        .await
        .map_err(|_| RedisError::MerkleTreeNotFound(block_number))?;

    // Redis [json_get] method returns a list of objects
    let mut tree_response: Vec<RawMerkleTree> = serde_json::from_str(&result).map_err(|e| {
        tracing::error!("Error while deserialzing: {e}");
        RedisError::TreeDeserialization
    })?;

    if tree_response.len() != 1 {
        return Err(RedisError::MerkleTreeNotFound(block_number));
    }

    // Safe to unwrap, see condition above
    let merkle_tree = MerkleTree::try_from(tree_response.pop().unwrap())
        .map_err(|_| RedisError::TreeDeserialization)?;

    // Update the cache with the merkle tree for the current block
    merkle_tree_cache
        .insert(block_number, merkle_tree.clone())
        .await;

    Ok(merkle_tree)
}

/// Converts a BlockId to a block number.
async fn get_block_number_from_id(
    redis_client: &Arc<redis::Client>,
    network: &Network,
    block_id: &BlockId,
) -> Result<u64, RedisError> {
    let block_number = match block_id {
        BlockId::Number(nbr) => *nbr,
        BlockId::Tag(tag) => get_block_number_for_tag(redis_client, network, tag).await?,
    };
    Ok(block_number)
}

/// Retrieve the block number corresponding to the block tag.
/// For us, the pending block is the latest block available in Redis,
/// and the latest is the one before.
async fn get_block_number_for_tag(
    redis_client: &Arc<redis::Client>,
    network: &Network,
    tag: &BlockTag,
) -> Result<u64, RedisError> {
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| RedisError::Connection)?;

    let key = format!("{}/latest_published_block", network);
    let latest_published_block: Option<u64> =
        conn.get(key).await.map_err(|_| RedisError::Connection)?;

    match latest_published_block {
        Some(latest) => match tag {
            BlockTag::Pending => Ok(latest),
            BlockTag::Latest => {
                if latest > 0 {
                    Ok(latest - 1)
                } else {
                    Err(RedisError::NoBlocks(network.to_string()))
                }
            }
        },
        None => Err(RedisError::NoBlocks(network.to_string())),
    }
}
