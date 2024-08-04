use color_eyre::{eyre::eyre, Result};
use reqwest::{Response, StatusCode};

use pragma_common::types::{
    merkle_tree::MerkleProof,
    options::{Instrument, OptionData},
    Network,
};

use crate::{constants::PRAGMAPI_PATH_PREFIX, types::MerkleFeedCalldata};

pub struct PragmaConsumer {
    pub(crate) network: Network,
    pub(crate) http_client: reqwest::Client,
    pub(crate) base_url: String,
}

impl PragmaConsumer {
    pub async fn get_deribit_options_calldata(
        &self,
        instrument: &Instrument,
        block_number: u64,
    ) -> Result<MerkleFeedCalldata> {
        let option_data = self.request_option(instrument.name(), block_number).await?;
        let merkle_proof = self
            .request_merkle_proof(option_data.hexadecimal_hash(), block_number)
            .await?;

        Ok(MerkleFeedCalldata {
            merkle_proof,
            option_data,
        })
    }

    async fn request_option(
        &self,
        instrument_name: String,
        block_number: u64,
    ) -> Result<OptionData> {
        let url = format!(
            "{}/{}/options/{}?network={}&block_number={}",
            self.base_url, PRAGMAPI_PATH_PREFIX, instrument_name, self.network, block_number,
        );

        tracing::info!("Url: {}", url);

        let api_response = self.request_api(url).await?;
        if api_response.status() != StatusCode::OK {
            return Err(eyre!("Request get_option failed!"));
        }

        let contents = api_response.text().await?;
        let option_data = serde_json::from_str(&contents)?;
        Ok(option_data)
    }

    async fn request_merkle_proof(
        &self,
        option_hash: String,
        block_number: u64,
    ) -> Result<MerkleProof> {
        let url = format!(
            "{}/{}/proof/{}?network={}&block_number={}",
            self.base_url, PRAGMAPI_PATH_PREFIX, option_hash, self.network, block_number,
        );

        let api_response = self.request_api(url).await?;
        if api_response.status() != StatusCode::OK {
            return Err(eyre!("Request get_proof failed!"));
        }

        let contents = api_response.text().await?;
        let merkle_proof = serde_json::from_str(&contents)?;
        Ok(merkle_proof)
    }

    async fn request_api(&self, url: String) -> Result<Response> {
        self.http_client
            .get(url)
            .send()
            .await
            .map_err(|e| eyre!("Request failed: {e}"))
    }
}
