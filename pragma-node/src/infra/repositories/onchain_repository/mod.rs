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
    utils::sql::get_interval_specifier,
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
        Err(e) => {
            tracing::error!("Could not get on-chain decimals for {pair}: {e}");
            0
        }
    };

    // Update cache with the new decimals
    if decimals != 0 {
        let network_decimals = decimals_cache.get(&network).await.unwrap_or_default();
        let mut updated_network_decimals = network_decimals.clone();
        updated_network_decimals.insert(pair_id, decimals);
        decimals_cache
            .insert(network, updated_network_decimals)
            .await;
    }

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
        (Network::Sepolia, DataType::SpotEntry) => "spot_candle",
        (Network::Mainnet, DataType::SpotEntry) => "mainnet_spot_candle",
        (Network::Sepolia, DataType::FutureEntry) => "perp_candle",
        (Network::Mainnet, DataType::FutureEntry) => "mainnet_perp_candle",
        _ => {
            return Err(InfraError::UnsupportedDataTypeForNetwork(
                network, data_type,
            ));
        }
    };
    let interval_specifier = match interval {
        Interval::TenSeconds => Ok("10_s"),
        Interval::OneMinute => Ok("1_min"),
        Interval::FiveMinutes => Ok("5_min"),
        Interval::FifteenMinutes => Ok("15_min"),
        Interval::OneHour => Ok("1_hour"),
        Interval::OneDay => Ok("1_day"),
        // We support less intervals for candles
        _ => Err(InfraError::UnsupportedOnchainInterval(interval)),
    }?;
    let table_name = format!("{prefix_name}_{interval_specifier}");
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
        (Network::Sepolia, DataType::SpotEntry) => "spot_median",
        (Network::Mainnet, DataType::SpotEntry) => "mainnet_spot_median",
        (Network::Sepolia, DataType::FutureEntry) => "perp_median",
        (Network::Mainnet, DataType::FutureEntry) => "mainnet_perp_median",
        _ => {
            return Err(InfraError::UnsupportedDataTypeForNetwork(
                network, data_type,
            ));
        }
    };

    let interval_specifier = get_interval_specifier(interval, false)?;
    let table_name = format!("{prefix_name}_{interval_specifier}");

    Ok(table_name)
}
