use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use starknet::providers::Provider;
use starknet::{
    core::{
        types::{BlockId, BlockTag, FunctionCall},
        utils::{cairo_short_string_to_felt, get_selector_from_name},
    },
    macros::felt_hex,
    providers::{JsonRpcClient, jsonrpc::HttpTransport},
};
use starknet_crypto::Felt;
use url::Url;

use pragma_common::types::{Network, pair::Pair};
use pragma_entities::InfraError;

pub const ENV_MAINNET_RPC_URL: &str = "MAINNET_RPC_URL";
pub const ENV_SEPOLIA_RPC_URL: &str = "SEPOLIA_RPC_URL";

pub type RpcClients = HashMap<Network, Arc<JsonRpcClient<HttpTransport>>>;

pub static ORACLE_ADDRESS_PER_NETWORK: LazyLock<HashMap<Network, Felt>> = LazyLock::new(|| {
    let mut addresses = HashMap::new();
    addresses.insert(
        Network::Mainnet,
        felt_hex!("0x2a85bd616f912537c50a49a4076db02c00b29b2cdc8a197ce92ed1837fa875b"),
    );
    addresses.insert(
        Network::Sepolia,
        felt_hex!("0x36031daa264c24520b11d93af622c848b2499b66b41d611bac95e13cfca131a"),
    );
    addresses
});

/// Init the RPC clients based on the provided ENV variables.
/// Panics if the env are not correctly set.
pub fn init_rpc_clients() -> HashMap<Network, Arc<JsonRpcClient<HttpTransport>>> {
    let mainnet_rpc_url: Url = std::env::var(ENV_MAINNET_RPC_URL)
        .unwrap_or("https://free-rpc.nethermind.io/mainnet-juno".to_string())
        .parse()
        .expect("Invalid MAINNET_RPC_URL provided.");
    let mainnet_client = JsonRpcClient::new(HttpTransport::new(mainnet_rpc_url));

    let sepolia_rpc_url: Url = std::env::var(ENV_SEPOLIA_RPC_URL)
        .unwrap_or("https://free-rpc.nethermind.io/sepolia-juno".to_string())
        .parse()
        .expect("Invalid SEPOLIA_RPC_URL provided.");
    let sepolia_client = JsonRpcClient::new(HttpTransport::new(sepolia_rpc_url));

    let mut rpc_clients = HashMap::new();
    rpc_clients.insert(Network::Mainnet, Arc::new(mainnet_client));
    rpc_clients.insert(Network::Sepolia, Arc::new(sepolia_client));

    rpc_clients
}

/// Calls the `get_decimals` endpoint of pragma oracle and returns the result.
pub async fn call_get_decimals(
    rpc_client: &Arc<JsonRpcClient<HttpTransport>>,
    pair: &Pair,
    network: Network,
) -> Result<u32, InfraError> {
    let pair_id = cairo_short_string_to_felt(&pair.to_pair_id())
        .map_err(|_| InfraError::PairNotFound(pair.to_pair_id()))?;
    let Some(pragma_oracle_address) = ORACLE_ADDRESS_PER_NETWORK.get(&network) else {
        unreachable!()
    };

    let request = FunctionCall {
        contract_address: *pragma_oracle_address,
        entry_point_selector: get_selector_from_name("get_decimals")
            .map_err(|e| InfraError::RpcError(e.to_string()))?,
        calldata: vec![Felt::ZERO, pair_id],
    };

    let call_result = rpc_client
        .call(request, BlockId::Tag(BlockTag::Pending))
        .await
        .map_err(|e| InfraError::RpcError(e.to_string()))?;

    let Some(felt_decimals) = call_result.first() else {
        return Err(InfraError::PairNotFound(pair.to_pair_id()));
    };

    let decimals: u32 = felt_decimals
        .to_biguint()
        .try_into()
        .map_err(|_| InfraError::RpcError(format!("Converting {felt_decimals} to Biguint")))?;

    Ok(decimals)
}
