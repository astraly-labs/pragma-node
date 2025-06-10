use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use starknet::core::types::{
    BlockHashAndNumber, BlockId, BroadcastedTransaction, ContractClass, EventFilter,
    FeeEstimate, Felt, FunctionCall, MaybePendingBlockWithTxHashes,
    MaybePendingBlockWithTxs, MaybePendingStateUpdate, MaybePendingTransactionReceipt,
    SimulatedTransaction, SimulationFlag, SyncStatusType, Transaction,
    TransactionExecutionStatus, TransactionFinalityStatus, TransactionReceiptWithBlockInfo,
    TransactionTraceWithHash,
};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Provider, ProviderError};
use tokio::time::timeout;
use url::Url;

/// The list of all the starknet rpcs that the FallbackProvider may use.
/// They're sorted by priority (so we sorted them by reliability here).
pub const STARKNET_MAINNET_RPC_URLS: [&str; 10] = [
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

/// The list of Sepolia testnet RPC URLs for testing
pub const STARKNET_SEPOLIA_RPC_URLS: [&str; 5] = [
    "https://starknet-sepolia.public.blastapi.io/rpc/v0_7",
    "https://starknet-sepolia.infura.io/v3/1e978c4df1984be09e18e5cd849228e4",
    "https://api.cartridge.gg/x/starknet/sepolia",
    "https://free-rpc.nethermind.io/sepolia-juno",
    "https://starknet-sepolia.blockpi.network/v1/rpc/public",
];

/// A provider that automatically falls back to alternative RPC endpoints when requests fail.
/// Implements the Provider trait and manages multiple JsonRpcClient instances internally.
pub struct FallbackProvider {
    providers: Vec<JsonRpcClient<HttpTransport>>,
    current_index: Arc<AtomicUsize>,
    timeout_duration: Duration,
}

impl FallbackProvider {
    /// Creates a new FallbackProvider with the given RPC URLs
    pub fn new(urls: Vec<&str>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::new_with_timeout(urls, Duration::from_secs(30))
    }

    /// Creates a new FallbackProvider with custom timeout
    pub fn new_with_timeout(
        urls: Vec<&str>,
        timeout_duration: Duration,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if urls.is_empty() {
            return Err("At least one RPC URL must be provided".into());
        }

        let providers = urls
            .into_iter()
            .map(|url| {
                let parsed_url = Url::parse(url)
                    .map_err(|e| format!("Invalid URL '{}': {}", url, e))?;
                Ok(JsonRpcClient::new(HttpTransport::new(parsed_url)))
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error + Send + Sync>>>()?;

        Ok(Self {
            providers,
            current_index: Arc::new(AtomicUsize::new(0)),
            timeout_duration,
        })
    }

    /// Creates a FallbackProvider for Mainnet using the predefined RPC URLs
    pub fn mainnet() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::new(STARKNET_MAINNET_RPC_URLS.to_vec())
    }

    /// Creates a FallbackProvider for Sepolia testnet using the predefined RPC URLs
    pub fn sepolia() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::new(STARKNET_SEPOLIA_RPC_URLS.to_vec())
    }

    /// Executes an operation with automatic fallback to alternative providers
    async fn execute_with_fallback<F, T, Fut>(&self, operation: F) -> Result<T, ProviderError>
    where
        F: Fn(&JsonRpcClient<HttpTransport>) -> Fut,
        Fut: Future<Output = Result<T, ProviderError>>,
    {
        let mut last_error = None;
        let start_index = self.current_index.load(Ordering::Relaxed);

        for i in 0..self.providers.len() {
            let provider_index = (start_index + i) % self.providers.len();
            let provider = &self.providers[provider_index];

            match timeout(self.timeout_duration, operation(provider)).await {
                Ok(Ok(result)) => {
                    // Update current index to the successful provider for next time
                    self.current_index.store(provider_index, Ordering::Relaxed);
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    tracing::warn!(
                        "Provider at index {} failed with error: {:?}",
                        provider_index,
                        e
                    );
                    last_error = Some(e);
                }
                Err(_) => {
                    let timeout_error = ProviderError::Other(format!(
                        "Provider at index {} timed out after {:?}",
                        provider_index, self.timeout_duration
                    ));
                    tracing::warn!("{}", timeout_error);
                    last_error = Some(timeout_error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| ProviderError::Other("All providers failed".into())))
    }

    /// Gets the number of configured providers
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Gets the current active provider index
    pub fn current_provider_index(&self) -> usize {
        self.current_index.load(Ordering::Relaxed)
    }
}

#[async_trait::async_trait]
impl Provider for FallbackProvider {
    async fn spec_version(&self) -> Result<String, ProviderError> {
        self.execute_with_fallback(|provider| provider.spec_version())
            .await
    }

    async fn get_block_with_tx_hashes<B>(
        &self,
        block_id: B,
    ) -> Result<MaybePendingBlockWithTxHashes, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let block_id = block_id.as_ref().clone();
        self.execute_with_fallback(move |provider| provider.get_block_with_tx_hashes(&block_id))
            .await
    }

    async fn get_block_with_txs<B>(
        &self,
        block_id: B,
    ) -> Result<MaybePendingBlockWithTxs, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let block_id = block_id.as_ref().clone();
        self.execute_with_fallback(move |provider| provider.get_block_with_txs(&block_id))
            .await
    }

    async fn get_state_update<B>(
        &self,
        block_id: B,
    ) -> Result<MaybePendingStateUpdate, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let block_id = block_id.as_ref().clone();
        self.execute_with_fallback(move |provider| provider.get_state_update(&block_id))
            .await
    }

    async fn get_storage_at<A, K, B>(
        &self,
        contract_address: A,
        key: K,
        block_id: B,
    ) -> Result<Felt, ProviderError>
    where
        A: AsRef<Felt> + Send + Sync,
        K: AsRef<Felt> + Send + Sync,
        B: AsRef<BlockId> + Send + Sync,
    {
        let contract_address = *contract_address.as_ref();
        let key = *key.as_ref();
        let block_id = block_id.as_ref().clone();
        self.execute_with_fallback(move |provider| {
            provider.get_storage_at(contract_address, key, &block_id)
        })
        .await
    }

    async fn get_transaction_status<H>(
        &self,
        transaction_hash: H,
    ) -> Result<TransactionFinalityStatus, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        let transaction_hash = *transaction_hash.as_ref();
        self.execute_with_fallback(move |provider| provider.get_transaction_status(transaction_hash))
            .await
    }

    async fn get_transaction_by_hash<H>(
        &self,
        transaction_hash: H,
    ) -> Result<Transaction, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        let transaction_hash = *transaction_hash.as_ref();
        self.execute_with_fallback(move |provider| provider.get_transaction_by_hash(transaction_hash))
            .await
    }

    async fn get_transaction_by_block_id_and_index<B>(
        &self,
        block_id: B,
        index: u64,
    ) -> Result<Transaction, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let block_id = block_id.as_ref().clone();
        self.execute_with_fallback(move |provider| {
            provider.get_transaction_by_block_id_and_index(&block_id, index)
        })
        .await
    }

    async fn get_transaction_receipt<H>(
        &self,
        transaction_hash: H,
    ) -> Result<MaybePendingTransactionReceipt, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        let transaction_hash = *transaction_hash.as_ref();
        self.execute_with_fallback(move |provider| provider.get_transaction_receipt(transaction_hash))
            .await
    }

    async fn get_class<B, H>(
        &self,
        block_id: B,
        class_hash: H,
    ) -> Result<ContractClass, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        H: AsRef<Felt> + Send + Sync,
    {
        let block_id = block_id.as_ref().clone();
        let class_hash = *class_hash.as_ref();
        self.execute_with_fallback(move |provider| provider.get_class(&block_id, class_hash))
            .await
    }

    async fn get_class_hash_at<B, A>(
        &self,
        block_id: B,
        contract_address: A,
    ) -> Result<Felt, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        let block_id = block_id.as_ref().clone();
        let contract_address = *contract_address.as_ref();
        self.execute_with_fallback(move |provider| {
            provider.get_class_hash_at(&block_id, contract_address)
        })
        .await
    }

    async fn get_class_at<B, A>(
        &self,
        block_id: B,
        contract_address: A,
    ) -> Result<ContractClass, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        let block_id = block_id.as_ref().clone();
        let contract_address = *contract_address.as_ref();
        self.execute_with_fallback(move |provider| provider.get_class_at(&block_id, contract_address))
            .await
    }

    async fn get_block_transaction_count<B>(&self, block_id: B) -> Result<u64, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let block_id = block_id.as_ref().clone();
        self.execute_with_fallback(move |provider| provider.get_block_transaction_count(&block_id))
            .await
    }

    async fn call<R, B>(&self, request: R, block_id: B) -> Result<Vec<Felt>, ProviderError>
    where
        R: AsRef<FunctionCall> + Send + Sync,
        B: AsRef<BlockId> + Send + Sync,
    {
        let request = request.as_ref().clone();
        let block_id = block_id.as_ref().clone();
        self.execute_with_fallback(move |provider| provider.call(&request, &block_id))
            .await
    }

    async fn estimate_fee<R, S, B>(
        &self,
        request: R,
        simulation_flags: S,
        block_id: B,
    ) -> Result<Vec<FeeEstimate>, ProviderError>
    where
        R: AsRef<[BroadcastedTransaction]> + Send + Sync,
        S: AsRef<[SimulationFlag]> + Send + Sync,
        B: AsRef<BlockId> + Send + Sync,
    {
        let request = request.as_ref().to_vec();
        let simulation_flags = simulation_flags.as_ref().to_vec();
        let block_id = block_id.as_ref().clone();
        self.execute_with_fallback(move |provider| {
            provider.estimate_fee(&request, &simulation_flags, &block_id)
        })
        .await
    }

    async fn estimate_message_fee<M, B>(
        &self,
        message: M,
        block_id: B,
    ) -> Result<FeeEstimate, ProviderError>
    where
        M: AsRef<starknet::core::types::MsgFromL1> + Send + Sync,
        B: AsRef<BlockId> + Send + Sync,
    {
        let message = message.as_ref().clone();
        let block_id = block_id.as_ref().clone();
        self.execute_with_fallback(move |provider| provider.estimate_message_fee(&message, &block_id))
            .await
    }

    async fn block_number(&self) -> Result<u64, ProviderError> {
        self.execute_with_fallback(|provider| provider.block_number())
            .await
    }

    async fn block_hash_and_number(&self) -> Result<BlockHashAndNumber, ProviderError> {
        self.execute_with_fallback(|provider| provider.block_hash_and_number())
            .await
    }

    async fn chain_id(&self) -> Result<Felt, ProviderError> {
        self.execute_with_fallback(|provider| provider.chain_id())
            .await
    }

    async fn syncing(&self) -> Result<SyncStatusType, ProviderError> {
        self.execute_with_fallback(|provider| provider.syncing())
            .await
    }

    async fn get_events(
        &self,
        filter: EventFilter,
        continuation_token: Option<String>,
        chunk_size: u64,
    ) -> Result<starknet::core::types::EventsPage, ProviderError> {
        self.execute_with_fallback(move |provider| {
            provider.get_events(filter.clone(), continuation_token.clone(), chunk_size)
        })
        .await
    }

    async fn get_nonce<B, A>(&self, block_id: B, contract_address: A) -> Result<Felt, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        let block_id = block_id.as_ref().clone();
        let contract_address = *contract_address.as_ref();
        self.execute_with_fallback(move |provider| provider.get_nonce(&block_id, contract_address))
            .await
    }

    async fn add_invoke_transaction<I>(
        &self,
        invoke_transaction: I,
    ) -> Result<starknet::core::types::InvokeTransactionResult, ProviderError>
    where
        I: AsRef<starknet::core::types::BroadcastedInvokeTransaction> + Send + Sync,
    {
        let invoke_transaction = invoke_transaction.as_ref().clone();
        self.execute_with_fallback(move |provider| {
            provider.add_invoke_transaction(&invoke_transaction)
        })
        .await
    }

    async fn add_declare_transaction<D>(
        &self,
        declare_transaction: D,
    ) -> Result<starknet::core::types::DeclareTransactionResult, ProviderError>
    where
        D: AsRef<starknet::core::types::BroadcastedDeclareTransaction> + Send + Sync,
    {
        let declare_transaction = declare_transaction.as_ref().clone();
        self.execute_with_fallback(move |provider| {
            provider.add_declare_transaction(&declare_transaction)
        })
        .await
    }

    async fn add_deploy_account_transaction<D>(
        &self,
        deploy_account_transaction: D,
    ) -> Result<starknet::core::types::DeployAccountTransactionResult, ProviderError>
    where
        D: AsRef<starknet::core::types::BroadcastedDeployAccountTransaction> + Send + Sync,
    {
        let deploy_account_transaction = deploy_account_transaction.as_ref().clone();
        self.execute_with_fallback(move |provider| {
            provider.add_deploy_account_transaction(&deploy_account_transaction)
        })
        .await
    }

    async fn trace_transaction<H>(
        &self,
        transaction_hash: H,
    ) -> Result<TransactionTraceWithHash, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        let transaction_hash = *transaction_hash.as_ref();
        self.execute_with_fallback(move |provider| provider.trace_transaction(transaction_hash))
            .await
    }

    async fn simulate_transactions<B, T, S>(
        &self,
        block_id: B,
        transactions: T,
        simulation_flags: S,
    ) -> Result<Vec<SimulatedTransaction>, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        T: AsRef<[BroadcastedTransaction]> + Send + Sync,
        S: AsRef<[SimulationFlag]> + Send + Sync,
    {
        let block_id = block_id.as_ref().clone();
        let transactions = transactions.as_ref().to_vec();
        let simulation_flags = simulation_flags.as_ref().to_vec();
        self.execute_with_fallback(move |provider| {
            provider.simulate_transactions(&block_id, &transactions, &simulation_flags)
        })
        .await
    }

    async fn trace_block_transactions<B>(
        &self,
        block_id: B,
    ) -> Result<Vec<TransactionTraceWithHash>, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let block_id = block_id.as_ref().clone();
        self.execute_with_fallback(move |provider| provider.trace_block_transactions(&block_id))
            .await
    }

    async fn get_transaction_receipt_by_block_id_and_index<B>(
        &self,
        block_id: B,
        index: u64,
    ) -> Result<TransactionReceiptWithBlockInfo, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let block_id = block_id.as_ref().clone();
        self.execute_with_fallback(move |provider| {
            provider.get_transaction_receipt_by_block_id_and_index(&block_id, index)
        })
        .await
    }

    async fn get_l1_message_hash<H>(&self, l2_tx_hash: H) -> Result<starknet::core::types::Hash256, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        let l2_tx_hash = *l2_tx_hash.as_ref();
        self.execute_with_fallback(move |provider| provider.get_l1_message_hash(l2_tx_hash))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_provider_creation() {
        let urls = vec!["https://example1.com", "https://example2.com"];
        let provider = FallbackProvider::new(urls);
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().provider_count(), 2);
    }

    #[test]
    fn test_fallback_provider_empty_urls() {
        let urls = vec![];
        let provider = FallbackProvider::new(urls);
        assert!(provider.is_err());
    }

    #[test]
    fn test_fallback_provider_invalid_url() {
        let urls = vec!["not-a-valid-url"];
        let provider = FallbackProvider::new(urls);
        assert!(provider.is_err());
    }

    #[test]
    fn test_predefined_providers() {
        let mainnet_provider = FallbackProvider::mainnet();
        assert!(mainnet_provider.is_ok());
        assert_eq!(
            mainnet_provider.unwrap().provider_count(),
            STARKNET_MAINNET_RPC_URLS.len()
        );

        let sepolia_provider = FallbackProvider::sepolia();
        assert!(sepolia_provider.is_ok());
        assert_eq!(
            sepolia_provider.unwrap().provider_count(),
            STARKNET_SEPOLIA_RPC_URLS.len()
        );
    }
}