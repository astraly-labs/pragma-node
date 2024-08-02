use pragma_common::types::Network;
use reqwest::{Response, StatusCode};

use crate::types::{Instrument, MerkleFeedCalldata, OptionData};

pub struct PragmaConsumer {
    pub(crate) network: Network,
    pub(crate) http_client: reqwest::Client,
    pub(crate) base_url: String,
}

impl PragmaConsumer {
    async fn request_api(&self, url: String) -> Response {
        self.http_client.get(url).send().await.unwrap()
    }

    async fn request_latest_option(&self, instrument_name: String) -> Result<OptionData, ()> {
        let url = format!(
            "{}/{}/get_latest_option&instrument={}",
            self.base_url, self.network, instrument_name,
        );

        let api_response = self.request_api(url).await;
        if api_response.status() != StatusCode::OK {
            return Err(());
        }

        Ok(OptionData::default())
    }

    async fn request_latest_merkle_tree(&self) -> Result<Vec<u8>, ()> {
        let url = format!("{}/{}/get_latest_merkle_tree", self.base_url, self.network);

        let api_response = self.request_api(url).await;
        if api_response.status() != StatusCode::OK {
            return Err(());
        }

        Ok(vec![])
    }

    pub async fn get_deribit_options_calldata(
        &self,
        instrument: &Instrument,
    ) -> Result<MerkleFeedCalldata, ()> {
        let _merkle_tree = self.request_latest_merkle_tree().await?;
        let _option = self.request_latest_option(instrument.name()).await?;

        Ok(MerkleFeedCalldata::default())
    }
}
