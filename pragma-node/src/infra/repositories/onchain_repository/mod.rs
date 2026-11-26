pub mod checkpoint;
pub mod entry;
pub mod history;
pub mod observations;
pub mod ohlc;
pub mod publisher;

use std::collections::HashMap;

use moka::future::Cache;

use pragma_common::starknet::StarknetNetwork;
use pragma_common::{InstrumentType, Interval, Pair};
use pragma_entities::error::InfraError;

use crate::{
    infra::rpc::{RpcClients, call_get_decimals},
    utils::sql::get_interval_specifier,
};

/// Retrieves the on-chain decimals for the provided network & pair.
pub(crate) async fn get_onchain_decimals(
    decimals_cache: &Cache<StarknetNetwork, HashMap<String, u32>>,
    rpc_clients: &RpcClients,
    network: StarknetNetwork,
    pair: &Pair,
) -> Result<u32, InfraError> {
    let pair_id = pair.to_pair_id();

    // Try to get decimals from cache first
    if let Some(network_decimals) = decimals_cache.get(&network).await
        && let Some(decimals) = network_decimals.get(&pair_id)
    {
        return Ok(*decimals);
    }

    // If not found in cache, call RPC
    let Some(rpc_client) = rpc_clients.get(&network) else {
        return Err(InfraError::NoRpcAvailable(network));
    };

    let decimals = call_get_decimals(rpc_client, pair, network).await?;

    // Update cache with the new decimals
    let mut network_decimals = decimals_cache.get(&network).await.unwrap_or_default();
    network_decimals.insert(pair_id, decimals);
    decimals_cache.insert(network, network_decimals).await;

    Ok(decimals)
}

/// Retrieve the onchain table name based on the network and data type.
pub(crate) const fn get_onchain_table_name(
    network: StarknetNetwork,
    data_type: InstrumentType,
) -> &'static str {
    match (network, data_type) {
        (StarknetNetwork::Sepolia, InstrumentType::Spot) => "spot_entry",
        (StarknetNetwork::Mainnet, InstrumentType::Spot) => "mainnet_spot_entry",
        (StarknetNetwork::Sepolia, InstrumentType::Perp) => "future_entry",
        (StarknetNetwork::Mainnet, InstrumentType::Perp) => "mainnet_future_entry",
    }
}

/// Retrieve the onchain table name for the OHLC based on network, datatype & interval.
pub(crate) fn get_onchain_ohlc_table_name(
    network: StarknetNetwork,
    data_type: InstrumentType,
    interval: Interval,
) -> Result<String, InfraError> {
    let prefix_name = match (network, data_type) {
        (StarknetNetwork::Sepolia, InstrumentType::Spot) => "spot_candle",
        (StarknetNetwork::Mainnet, InstrumentType::Spot) => "mainnet_spot_candle",
        (StarknetNetwork::Sepolia, InstrumentType::Perp) => "perp_candle",
        (StarknetNetwork::Mainnet, InstrumentType::Perp) => "mainnet_perp_candle",
    };
    let interval_specifier = match interval {
        Interval::TenSeconds => Ok("10_s"),
        Interval::OneMinute => Ok("1_min"),
        Interval::FiveMinutes => Ok("5_min"),
        Interval::FifteenMinutes => Ok("15_min"),
        Interval::OneHour => Ok("1_h"),
        Interval::OneDay => Ok("1_day"),
        Interval::OneWeek => Ok("1_week"),
        // We support less intervals for candles
        _ => Err(InfraError::UnsupportedOnchainInterval(interval)),
    }?;
    let table_name = format!("{prefix_name}_{interval_specifier}");
    Ok(table_name)
}

/// Retrieve the onchain table name for Timescale aggregates (medians) based on the network,
/// datatype & interval.
pub(crate) fn get_onchain_aggregate_table_name(
    network: StarknetNetwork,
    data_type: InstrumentType,
    interval: Interval,
) -> Result<String, InfraError> {
    let prefix_name = match (network, data_type) {
        (StarknetNetwork::Sepolia, InstrumentType::Spot) => "spot_median",
        (StarknetNetwork::Mainnet, InstrumentType::Spot) => "mainnet_spot_median",
        (StarknetNetwork::Sepolia, InstrumentType::Perp) => "perp_median",
        (StarknetNetwork::Mainnet, InstrumentType::Perp) => "mainnet_perp_median",
    };

    let interval_specifier = get_interval_specifier(interval, false)?;
    let table_name = format!("{prefix_name}_{interval_specifier}");

    Ok(table_name)
}

#[cfg(test)]
mod tests {
    use axum::{Router, http::StatusCode, routing::post};
    use pragma_common::starknet::fallback_provider::FallbackProvider;
    use url::Url;

    use super::*;

    #[rstest::rstest]
    #[case("0x8", 8)]
    #[case("0x6", 6)]
    #[case("0x0", 0)]
    #[tokio::test]
    async fn decimals_are_read_and_cached(#[case] result: &str, #[case] expected: u32) {
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": [result]});
        let app = Router::new().route("/", post(move || async move { axum::Json(body) }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let network = StarknetNetwork::Mainnet;
        let clients = HashMap::from([(network, FallbackProvider::new(vec![url]).unwrap())]);
        let cache = Cache::new(1);
        let pair = Pair::try_from("BTC/USD").unwrap();

        let decimals = get_onchain_decimals(&cache, &clients, network, &pair)
            .await
            .unwrap();
        server.abort();
        assert_eq!(decimals, expected);
        assert_eq!(
            cache.get(&network).await.unwrap().get("BTC/USD"),
            Some(&expected)
        );
        assert_eq!(
            get_onchain_decimals(&cache, &HashMap::new(), network, &pair)
                .await
                .unwrap(),
            expected
        );
    }

    #[tokio::test]
    async fn failed_decimal_lookup_is_not_a_zero_decimal_price() {
        let app = Router::new().route(
            "/",
            post(|| async { (StatusCode::UNAUTHORIZED, "Unauthorized") }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let network = StarknetNetwork::Mainnet;
        let clients = HashMap::from([(network, FallbackProvider::new(vec![url]).unwrap())]);
        let cache = Cache::new(1);

        let result = get_onchain_decimals(
            &cache,
            &clients,
            network,
            &Pair::try_from("BTC/USD").unwrap(),
        )
        .await;
        server.abort();
        assert!(matches!(result, Err(InfraError::RpcError(_))));
        assert!(cache.get(&network).await.is_none());
    }
}
