use std::{collections::HashMap, sync::LazyLock};

use starknet::providers::Provider;
use starknet::{
    core::{
        types::{BlockId, BlockTag, FunctionCall},
        utils::{cairo_short_string_to_felt, get_selector_from_name},
    },
    macros::felt_hex,
};
use starknet_crypto::Felt;
use url::Url;

use pragma_common::{
    Pair,
    starknet::{StarknetNetwork, fallback_provider::FallbackProvider},
};
use pragma_entities::InfraError;

pub const ENV_MAINNET_RPC_URL: &str = "MAINNET_RPC_URL";
pub const ENV_SEPOLIA_RPC_URL: &str = "SEPOLIA_RPC_URL";

pub type RpcClients = HashMap<StarknetNetwork, FallbackProvider>;

pub static ORACLE_ADDRESS_PER_NETWORK: LazyLock<HashMap<StarknetNetwork, Felt>> =
    LazyLock::new(|| {
        let mut addresses = HashMap::new();
        addresses.insert(
            StarknetNetwork::Mainnet,
            felt_hex!("0x2a85bd616f912537c50a49a4076db02c00b29b2cdc8a197ce92ed1837fa875b"),
        );
        addresses.insert(
            StarknetNetwork::Sepolia,
            felt_hex!("0x36031daa264c24520b11d93af622c848b2499b66b41d611bac95e13cfca131a"),
        );
        addresses
    });

/// The list of all the starknet rpcs that the FallbackProvider may use.
/// They're sorted by priority (so we sorted them by reliability here).
pub const MAINNET_STARKNET_RPC_URLS: [&str; 10] = [
    "https://api.cartridge.gg/x/starknet/mainnet",
    "https://starknet-mainnet.g.alchemy.com/starknet/version/rpc/v0_8/WrkE4HqPXT-zi7gQn8bUtH-TXgYYs3w1",
    "https://starknet-mainnet.blastapi.io/d4c81751-861c-4970-bef5-9decd7f7aa39",
    "https://starknet-mainnet.infura.io/v3/1e978c4df1984be09e18e5cd849228e4",
    "https://mainnet-pragma.karnot.xyz/",
    "https://api.zan.top/public/starknet-mainnet",
    "https://starknet.api.onfinality.io/public",
    "https://rpc.starknet.lava.build:443",
    "https://starknet-mainnet.reddio.com",
    "https://starknet.drpc.org",
];

pub const SEPOLIA_STARKNET_RPC_URLS: [&str; 6] = [
    "https://api.cartridge.gg/x/starknet/sepolia",
    "https://sepolia-pragma.karnot.xyz/",
    "https://rpc.starknet-testnet.lava.build:443",
    "https://starknet-sepolia.reddio.com",
    "https://starknet-sepolia.drpc.org",
    "https://starknet-sepolia.public.blastapi.io",
];

/// Init the RPC clients based on the provided ENV variables.
/// Panics if the env are not correctly set.
pub fn init_rpc_clients() -> HashMap<StarknetNetwork, FallbackProvider> {
    let mainnet_starknet_provider = FallbackProvider::new(
        MAINNET_STARKNET_RPC_URLS
            .iter()
            .map(|url| Url::parse(url).unwrap())
            .collect(),
    )
    .expect("Could not init the starknet provider");

    let sepolia_starknet_provider = FallbackProvider::new(
        SEPOLIA_STARKNET_RPC_URLS
            .iter()
            .map(|url| Url::parse(url).unwrap())
            .collect(),
    )
    .expect("Could not init the starknet provider");

    let mut rpc_clients = HashMap::new();
    rpc_clients.insert(StarknetNetwork::Mainnet, mainnet_starknet_provider);
    rpc_clients.insert(StarknetNetwork::Sepolia, sepolia_starknet_provider);

    rpc_clients
}

/// Calls the `get_decimals` endpoint of pragma oracle and returns the result.
pub async fn call_get_decimals(
    rpc_client: &FallbackProvider,
    pair: &Pair,
    network: StarknetNetwork,
) -> Result<u32, InfraError> {
    let pair_id = cairo_short_string_to_felt(&pair.to_pair_id())
        .map_err(|_| InfraError::PairNotFound(pair.to_pair_id()))?;

    let Some(pragma_oracle_address) = ORACLE_ADDRESS_PER_NETWORK.get(&network) else {
        unreachable!()
    };

    let request = FunctionCall {
        contract_address: *pragma_oracle_address,
        entry_point_selector: get_selector_from_name("get_decimals")
            .map_err(|e| InfraError::RpcError(format!("{e:?}")))?,
        calldata: vec![Felt::ZERO, pair_id],
    };

    let call_result = rpc_client
        .call(request, BlockId::Tag(BlockTag::Pending))
        .await
        .map_err(|e| InfraError::RpcError(format!("{e:?}")))?;

    let Some(felt_decimals) = call_result.first() else {
        return Err(InfraError::PairNotFound(pair.to_pair_id()));
    };

    let decimals: u32 = felt_decimals
        .to_biguint()
        .try_into()
        .map_err(|_| InfraError::RpcError(format!("Converting {felt_decimals} to Biguint")))?;

    Ok(decimals)
}
