use color_eyre::{eyre::eyre, Result};
use reqwest::{Response, StatusCode};

use pragma_common::types::Network;

use crate::{
    constants::PRAGMAPI_PATH_PREFIX,
    types::{Instrument, MerkleFeedCalldata, OptionData},
};

pub struct PragmaConsumer {
    pub(crate) network: Network,
    pub(crate) http_client: reqwest::Client,
    pub(crate) base_url: String,
}

impl PragmaConsumer {
    pub async fn get_deribit_options_calldata(
        &self,
        instrument: &Instrument,
    ) -> Result<MerkleFeedCalldata> {
        let _merkle_tree = self.request_latest_merkle_tree().await?;
        // TODO: Change how options are stored redis so we can call one option
        let _option = self.request_latest_option(instrument.name()).await?;

        Ok(MerkleFeedCalldata::default())
    }

    async fn request_latest_option(&self, instrument_name: String) -> Result<OptionData> {
        // TODO: Create the get_latest_option endpoint.
        // TODO: Update the Redis storage so it's easy to query for an instrument name.
        let url = format!(
            "{}/{}/get_latest_option?network={}&instrument={}",
            self.base_url, PRAGMAPI_PATH_PREFIX, self.network, instrument_name,
        );

        let api_response = self.request_api(url).await?;
        if api_response.status() != StatusCode::OK {
            return Err(eyre!("Request get_latest_option failed!"));
        }

        // TODO: Serialization redis -> our type
        Ok(OptionData::default())
    }

    async fn request_latest_merkle_tree(&self) -> Result<Vec<u8>> {
        // TODO: Create the get_latest_option endpoint.
        let url = format!(
            "{}/{}/get_latest_merkle_tree?network={}",
            self.base_url, PRAGMAPI_PATH_PREFIX, self.network,
        );

        let api_response = self.request_api(url).await?;
        if api_response.status() != StatusCode::OK {
            return Err(eyre!("Request get_latest_merkle_tree failed!"));
        }

        // TODO: Serialization redis -> our type
        Ok(vec![])
    }

    async fn request_api(&self, url: String) -> Result<Response> {
        self.http_client
            .get(url)
            .send()
            .await
            .map_err(|e| eyre!("Request failed: {e}"))
    }
}
