use pragma_common::types::Network;
use reqwest::{
    header::{HeaderValue, InvalidHeaderValue},
    StatusCode,
};

use crate::{
    config::{ApiConfig, PragmaBaseUrl},
    constants::PRAGMAPI_HEALTHCHECK_ENDPOINT,
    consumer::PragmaConsumer,
};

#[derive(thiserror::Error, Debug)]
pub enum BuilderError {
    #[error("HTTP request to the pragmAPI failed with status `{0}`")]
    HttpRequest(StatusCode),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("unexpected health check response: `{0}`")]
    HealthCheck(String),
    #[error(transparent)]
    Header(#[from] InvalidHeaderValue),
}

/// Builder of the Pragma consumer client.
/// Default network is Sepolia.
#[derive(Default, Debug)]
pub struct PragmaConsumerBuilder {
    network: Network,
    check_api_health: bool,
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

    /// Perform an health check with the PragmAPI to make sur the connection is
    /// successfuly established.
    pub fn check_api_health(mut self) -> Self {
        self.check_api_health = true;
        self
    }

    pub async fn with_http(self, api_config: ApiConfig) -> Result<PragmaConsumer, BuilderError> {
        let http_client = self.build_http_client(&api_config)?;

        if self.check_api_health {
            self.http_health_check(&http_client, &api_config.base_url)
                .await?;
        }

        Ok(PragmaConsumer {
            network: self.network,
            http_client,
            base_url: api_config.base_url,
        })
    }

    fn build_http_client(&self, api_config: &ApiConfig) -> Result<reqwest::Client, BuilderError> {
        Ok(reqwest::Client::builder()
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    "x-api-key",
                    HeaderValue::from_str(&api_config.api_key).map_err(BuilderError::Header)?,
                );
                headers
            })
            .build()?)
    }

    async fn http_health_check(
        &self,
        client: &reqwest::Client,
        base_url: &PragmaBaseUrl,
    ) -> Result<(), BuilderError> {
        let health_check_url = format!("{}/{}", base_url.url(), PRAGMAPI_HEALTHCHECK_ENDPOINT);
        let response = client
            .get(&health_check_url)
            .send()
            .await
            .map_err(BuilderError::Reqwest)?;

        if response.status() != StatusCode::OK {
            return Err(BuilderError::HttpRequest(response.status()));
        }

        let body = response.text().await?;
        if body.trim() != "Server is running!" {
            return Err(BuilderError::HealthCheck(body));
        }

        Ok(())
    }
}
