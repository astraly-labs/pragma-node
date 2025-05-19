use std::sync::Arc;

use deadpool_diesel::postgres::Pool;
use starknet::signers::SigningKey;

use crate::caches::CacheRegistry;
use crate::infra::rpc::RpcClients;
use crate::metrics::MetricsRegistry;

#[derive(Clone)]
pub struct AppState {
    // Databases pools
    pub offchain_pool: Pool,
    pub onchain_pool: Pool,
    // Starknet RPC clients for mainnet & sepolia
    pub rpc_clients: RpcClients,
    // Database caches
    pub caches: Arc<CacheRegistry>,
    // Pragma Signer used for StarkEx signing
    pub pragma_signer: Option<SigningKey>,
    // Metrics
    pub metrics: Arc<MetricsRegistry>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("caches", &self.caches)
            .field("pragma_signer", &self.pragma_signer)
            .field("metrics", &self.metrics)
            .finish_non_exhaustive()
    }
}
