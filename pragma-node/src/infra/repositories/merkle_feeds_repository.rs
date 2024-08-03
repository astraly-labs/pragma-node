use std::sync::Arc;

use redis::JsonAsyncCommands;

use pragma_common::types::{merkle_tree::MerkleTree, options::OptionData, Network};
use pragma_entities::InfraError;

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

    let parsed: serde_json::Value =
        serde_json::from_str(&result).map_err(|_| InfraError::InternalServerError)?;

    let option_data = parsed.get(0).ok_or(InfraError::NotFound)?;

    let option_data: OptionData =
        serde_json::from_value(option_data.clone()).map_err(|_| InfraError::InternalServerError)?;

    Ok(option_data)
}

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

    let parsed: serde_json::Value = serde_json::from_str(&result).map_err(|e| {
        tracing::error!("Failed to parse JSON: {}", e);
        InfraError::InternalServerError
    })?;

    let merkle_tree_map = parsed.get(0).and_then(|v| v.as_object()).ok_or_else(|| {
        tracing::error!("Unexpected JSON structure");
        InfraError::InternalServerError
    })?;

    tracing::info!("{:?}", merkle_tree_map);

    Err(InfraError::NotFound)
}
