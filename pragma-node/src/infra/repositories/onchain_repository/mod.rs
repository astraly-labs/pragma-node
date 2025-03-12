pub mod checkpoint;
pub mod entry;
pub mod history;
pub mod ohlc;
pub mod publisher;

use std::collections::HashMap;

use moka::future::Cache;

use pragma_common::types::{DataType, Interval, Network, pair::Pair};
use pragma_entities::error::InfraError;

use crate::{
    infra::rpc::{RpcClients, call_get_decimals},
    is_enum_variant,
};

/// Retrieves the on-chain decimals for the provided network & pair.
pub(crate) async fn get_onchain_decimals(
    decimals_cache: &Cache<Network, HashMap<String, u32>>,
    rpc_clients: &RpcClients,
    network: Network,
    pair: &Pair,
) -> Result<u32, InfraError> {
    let pair_id = pair.to_pair_id();

    // Try to get decimals from cache first
    if let Some(network_decimals) = decimals_cache.get(&network).await {
        if let Some(decimals) = network_decimals.get(&pair_id) {
            return Ok(*decimals);
        }
    }

    // If not found in cache, call RPC
    let Some(rpc_client) = rpc_clients.get(&network) else {
        return Err(InfraError::NoRpcAvailable(network));
    };
    let decimals = match call_get_decimals(rpc_client, pair, network).await {
        Ok(decimals) => decimals,
        // TODO: we return 0 cause some pairs are failing when called in the contract and we want
        // to know which.
        Err(_) => 0,
    };

    // Update cache with the new decimals
    let network_decimals = decimals_cache.get(&network).await.unwrap_or_default();
    let mut updated_network_decimals = network_decimals.clone();
    updated_network_decimals.insert(pair_id, decimals);

    // Insert updated cache
    decimals_cache
        .insert(network, updated_network_decimals)
        .await;

    Ok(decimals)
}

/// Retrieve the onchain table name based on the network and data type.
pub(crate) const fn get_onchain_table_name(
    network: Network,
    data_type: DataType,
) -> Result<&'static str, InfraError> {
    let table = match (network, data_type) {
        (Network::Sepolia, DataType::SpotEntry) => "spot_entry",
        (Network::Mainnet, DataType::SpotEntry) => "mainnet_spot_entry",
        (Network::Sepolia, DataType::FutureEntry) => "future_entry",
        (Network::Mainnet, DataType::FutureEntry) => "mainnet_future_entry",
        _ => {
            return Err(InfraError::UnsupportedDataTypeForNetwork(
                network, data_type,
            ));
        }
    };
    Ok(table)
}

/// Retrieve the onchain table name for the OHLC based on network, datatype & interval.
pub(crate) fn get_onchain_ohlc_table_name(
    network: Network,
    data_type: DataType,
    interval: Interval,
) -> Result<String, InfraError> {
    let prefix_name = match (network, data_type) {
        (Network::Sepolia, DataType::SpotEntry) => "spot",
        (Network::Mainnet, DataType::SpotEntry) => "mainnet_spot",
        (Network::Sepolia, DataType::FutureEntry) => "future",
        (Network::Mainnet, DataType::FutureEntry) => "mainnet_future",
        _ => {
            return Err(InfraError::UnsupportedDataTypeForNetwork(
                network, data_type,
            ));
        }
    };
    let interval_specifier = get_onchain_interval_specifier(interval)?;
    let table_name = format!("{prefix_name}_{interval_specifier}_candle");
    Ok(table_name)
}

/// Retrieve the onchain table name for Timescale aggregates (medians) based on the network,
/// datatype & interval.
pub(crate) fn get_onchain_aggregate_table_name(
    network: Network,
    data_type: DataType,
    interval: Interval,
) -> Result<String, InfraError> {
    let prefix_name = match (network, data_type) {
        (Network::Sepolia, DataType::SpotEntry) => "spot_price",
        (Network::Mainnet, DataType::SpotEntry) => "mainnet_spot_price",
        (Network::Sepolia, DataType::FutureEntry) => "future_price",
        (Network::Mainnet, DataType::FutureEntry) => "mainnet_future_price",
        _ => {
            return Err(InfraError::UnsupportedDataTypeForNetwork(
                network, data_type,
            ));
        }
    };

    // NOTE: Special case because there is a mistake and we forgot the "s" on 2_hour
    let interval_specifier = if is_enum_variant!(interval, Interval::TwoHours) {
        "2_hour"
    } else {
        get_onchain_interval_specifier(interval)?
    };

    let table_name = format!("{prefix_name}_{interval_specifier}_agg");
    Ok(table_name)
}

pub const fn get_onchain_interval_specifier(
    interval: Interval,
) -> Result<&'static str, InfraError> {
    match interval {
        Interval::OneMinute => Ok("1_min"),
        Interval::FifteenMinutes => Ok("15_min"),
        Interval::OneHour => Ok("1_hour"),
        Interval::TwoHours => Ok("2_hour"),
        Interval::OneDay => Ok("1_day"),
        Interval::OneWeek => Ok("1_week"),
        _ => return Err(InfraError::UnsupportedOnchainInterval(interval)),
    }
}
