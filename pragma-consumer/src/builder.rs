use pragma_common::types::Network;

use crate::{client::PragmaConsumer, config::ApiConfig};

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

    pub fn with_api(
        self,
        api_config: ApiConfig,
    ) -> Result<PragmaConsumer, Box<dyn std::error::Error>> {
        let http_client = self.build_http_client(&api_config)?;
        Ok(PragmaConsumer {
            network: self.network,
            http_client,
            base_url: api_config.base_url,
        })
    }

    fn build_http_client(
        &self,
        api_config: &ApiConfig,
    ) -> Result<reqwest::Client, Box<dyn std::error::Error>> {
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
}
