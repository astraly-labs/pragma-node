use color_eyre::{eyre::eyre, Result};

use pragma_common::types::Network;
use reqwest::StatusCode;

use crate::{
    config::ApiConfig, constants::PRAGMAPI_HEALTHCHECK_ENDPOINT, consumer::PragmaConsumer,
};

/// Builder of the Pragma consumer client.
/// Default network is Sepolia.
#[derive(Default, Debug)]
pub struct PragmaConsumerBuilder {
    network: Network,
}

impl PragmaConsumerBuilder {
    pub fn new() -> Self {
        PragmaConsumerBuilder::default()
    }

    pub fn on_mainnet(self) -> Self {
        self.on_network(Network::Mainnet)
    }

    pub fn on_sepolia(self) -> Self {
        self.on_network(Network::Sepolia)
    }

    fn on_network(mut self, network: Network) -> Self {
        self.network = network;
        self
    }

    pub async fn with_api(self, api_config: ApiConfig) -> Result<PragmaConsumer> {
        let http_client = self.build_http_client(&api_config)?;

        // TODO(akhercha): Do we really want to make this health check?
        // Should just be an opt-in function?
        self.health_check(&http_client, &api_config.base_url)
            .await?;

        Ok(PragmaConsumer {
            network: self.network,
            http_client,
            base_url: api_config.base_url,
        })
    }

    fn build_http_client(&self, api_config: &ApiConfig) -> Result<reqwest::Client> {
        Ok(reqwest::Client::builder()
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!(
                        "X-API-KEY: {}",
                        api_config.api_key
                    ))?,
                );
                headers
            })
            .build()?)
    }

    async fn health_check(&self, client: &reqwest::Client, base_url: &str) -> Result<()> {
        let health_check_url = format!("{}/{}", base_url, PRAGMAPI_HEALTHCHECK_ENDPOINT);
        let response = client
            .get(&health_check_url)
            .send()
            .await
            .map_err(|e| eyre!("Could not reach URL \"{base_url}\": {e}"))?;

        if response.status() != StatusCode::OK {
            return Err(eyre!(
                "Health check failed: HTTP status {}",
                response.status()
            ));
        }

        let body = response.text().await?;
        if body.trim() != "Server is running!" {
            return Err(eyre!("Health check failed: Unexpected response body"));
        }

        Ok(())
    }
}
