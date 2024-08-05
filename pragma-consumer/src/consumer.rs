use reqwest::{Response, StatusCode};

use pragma_common::types::{
    merkle_tree::MerkleProof,
    options::{Instrument, OptionData},
    Network,
};

use crate::{constants::PRAGMAPI_PATH_PREFIX, types::MerkleFeedCalldata};

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
    pub(crate) base_url: String,
}

impl PragmaConsumer {
    /// Query the PragmAPI and returns the necessary calldata to use
    /// with our Oracle contract.
    pub async fn get_merkle_feed_calldata(
        &self,
        instrument: &Instrument,
        block_number: u64,
    ) -> Result<MerkleFeedCalldata, ConsumerError> {
        let option_data = self.request_option(instrument.name(), block_number).await?;
        let option_hash = option_data
            .pedersen_hash_as_hex_string()
            .map_err(|_| ConsumerError::OptionHash(option_data.clone()))?;

        let merkle_proof = self.request_merkle_proof(option_hash, block_number).await?;

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
        block_number: u64,
    ) -> Result<OptionData, ConsumerError> {
        let url = format!(
            "{}/{}/options/{}?network={}&block_number={}",
            self.base_url, PRAGMAPI_PATH_PREFIX, instrument_name, self.network, block_number,
        );

        let api_response = self.request_api(url).await?;
        if api_response.status() != StatusCode::OK {
            return Err(ConsumerError::HttpRequest(api_response.status()));
        }

        let contents = api_response.text().await.map_err(ConsumerError::Reqwest)?;
        let option_data = serde_json::from_str(&contents).map_err(ConsumerError::Serde)?;
        Ok(option_data)
    }

    /// Requests from our PragmAPI the merkle proof for an hash at a certain block.
    async fn request_merkle_proof(
        &self,
        option_hash: String,
        block_number: u64,
    ) -> Result<MerkleProof, ConsumerError> {
        let url = format!(
            "{}/{}/proof/{}?network={}&block_number={}",
            self.base_url, PRAGMAPI_PATH_PREFIX, option_hash, self.network, block_number,
        );

        let api_response = self.request_api(url).await?;
        if api_response.status() != StatusCode::OK {
            return Err(ConsumerError::HttpRequest(api_response.status()));
        }

        let contents = api_response.text().await.map_err(ConsumerError::Reqwest)?;
        let merkle_proof = serde_json::from_str(&contents).map_err(ConsumerError::Serde)?;
        Ok(merkle_proof)
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
