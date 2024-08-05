use reqwest::{Response, StatusCode};

use pragma_common::types::{
    block_id::{BlockId, BlockTag},
    merkle_tree::MerkleProof,
    options::{Instrument, OptionData},
    Network,
};

use crate::{config::PragmaBaseUrl, constants::PRAGMAPI_PATH_PREFIX, types::MerkleFeedCalldata};

#[derive(thiserror::Error, Debug)]
pub enum ConsumerError {
    #[error("http request to the pragmAPI failed with status `{0}`")]
    HttpRequest(StatusCode),
    #[error("could not decode the HTTP response: `{0}`")]
    Decode(String),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("could not compute the pedersen hash for option: `{:?}`", 0)]
    OptionHash(OptionData),
}

pub struct PragmaConsumer {
    pub(crate) network: Network,
    pub(crate) http_client: reqwest::Client,
    pub(crate) base_url: PragmaBaseUrl,
}

impl PragmaConsumer {
    /// Query the PragmAPI and returns the necessary calldata to use
    /// with our Oracle contract.
    pub async fn get_merkle_feed_calldata(
        &self,
        instrument: &Instrument,
        block_id: Option<BlockId>,
    ) -> Result<MerkleFeedCalldata, ConsumerError> {
        let block_id = block_id.unwrap_or(BlockId::Tag(BlockTag::Latest));
        let option_data = self.request_option(instrument.name(), block_id).await?;
        let option_hash = option_data
            .pedersen_hash_as_hex_string()
            .map_err(|_| ConsumerError::OptionHash(option_data.clone()))?;

        let merkle_proof = self.request_merkle_proof(option_hash, block_id).await?;

        Ok(MerkleFeedCalldata {
            merkle_proof,
            option_data,
        })
    }

    /// Requests from our PragmAPI the option data for a given instrument name at a
    /// certain block.
    async fn request_option(
        &self,
        instrument_name: String,
        block_id: BlockId,
    ) -> Result<OptionData, ConsumerError> {
        let url = format!(
            "{}/{}/options/{}?network={}&block_id={}",
            self.base_url.url(),
            PRAGMAPI_PATH_PREFIX,
            instrument_name,
            self.network,
            block_id,
        );

        let api_response = self.request_api(url).await?;
        if api_response.status() != StatusCode::OK {
            return Err(ConsumerError::HttpRequest(api_response.status()));
        }

        let contents = api_response.text().await.map_err(ConsumerError::Reqwest)?;
        serde_json::from_str(&contents).map_err(ConsumerError::Serde)
    }

    /// Requests from our PragmAPI the merkle proof for an hash at a certain block.
    async fn request_merkle_proof(
        &self,
        option_hash: String,
        block_id: BlockId,
    ) -> Result<MerkleProof, ConsumerError> {
        let url = format!(
            "{}/{}/proof/{}?network={}&block_id={}",
            self.base_url.url(),
            PRAGMAPI_PATH_PREFIX,
            option_hash,
            self.network,
            block_id,
        );

        let api_response = self.request_api(url).await?;
        if api_response.status() != StatusCode::OK {
            return Err(ConsumerError::HttpRequest(api_response.status()));
        }

        let contents = api_response.text().await.map_err(ConsumerError::Reqwest)?;
        serde_json::from_str(&contents).map_err(ConsumerError::Serde)
    }

    /// Utility function to make an HTTP Get request to a provided URL.
    async fn request_api(&self, url: String) -> Result<Response, ConsumerError> {
        self.http_client
            .get(url)
            .send()
            .await
            .map_err(ConsumerError::Reqwest)
    }
}
