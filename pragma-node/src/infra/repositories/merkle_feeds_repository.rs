use std::sync::Arc;

use redis::JsonAsyncCommands;

use pragma_common::types::{options::OptionData, Network};
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
