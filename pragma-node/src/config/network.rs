use starknet::{
    core::types::FieldElement,
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
};
use std::str::FromStr;
use std::sync::Arc;
use strum::{EnumString, IntoStaticStr};
use url::Url;

pub const MAINNET_ORACLE_ADDRESS: &str =
    "0x2a85bd616f912537c50a49a4076db02c00b29b2cdc8a197ce92ed1837fa875b";

pub const TESTNET_ORACLE_ADDRESS: &str =
    "0x36031daa264c24520b11d93af622c848b2499b66b41d611bac95e13cfca131a";

#[derive(Debug, Clone, EnumString, IntoStaticStr)]
pub enum NetworkName {
    #[strum(ascii_case_insensitive)]
    Mainnet,
    #[strum(ascii_case_insensitive)]
    Testnet,
}

impl Default for NetworkName {
    fn default() -> Self {
        NetworkName::Testnet
    }
}

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub name: NetworkName,
    pub oracle_address: FieldElement,
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
}

pub const ENV_NETWORK: &str = "NETWORK";
pub const ENV_RPC_URL: &str = "RPC_URL";

impl NetworkConfig {
    pub fn from_env() -> NetworkConfig {
        let network_name = std::env::var(ENV_NETWORK).unwrap_or_default();
        let network_name = NetworkName::from_str(&network_name).expect("Invalid network name");

        let oracle_address = match network_name {
            NetworkName::Mainnet => FieldElement::from_str(MAINNET_ORACLE_ADDRESS),
            NetworkName::Testnet => FieldElement::from_str(TESTNET_ORACLE_ADDRESS),
        };
        let oracle_address = oracle_address.expect("Could not parse oracle address");

        let rpc_url = std::env::var(ENV_RPC_URL).expect("RPC_URL must be set");
        let rpc_url = Url::parse(&rpc_url).expect("Invalid RPC URL");
        let rpc_client = JsonRpcClient::new(HttpTransport::new(rpc_url));

        NetworkConfig {
            name: network_name,
            oracle_address,
            provider: Arc::new(rpc_client),
        }
    }
}
