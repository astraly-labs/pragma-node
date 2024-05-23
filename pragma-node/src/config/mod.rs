pub mod network;
pub mod settings;

use dotenvy::dotenv;
use tokio::sync::OnceCell;

use network::NetworkConfig;
use settings::{KafkaConfig, ServerConfig};

#[derive(Debug)]
pub struct Config {
    server: ServerConfig,
    kafka: KafkaConfig,
    #[allow(dead_code)]
    network: NetworkConfig,
}

impl Config {
    pub fn server_host(&self) -> &str {
        &self.server.host
    }

    pub fn server_port(&self) -> u16 {
        self.server.port
    }

    pub fn kafka_topic(&self) -> &str {
        &self.kafka.topic
    }

    pub fn network(&self) -> NetworkConfig {
        self.network.clone()
    }
}

pub static CONFIG: OnceCell<Config> = OnceCell::const_new();

async fn init_config() -> Config {
    dotenv().ok();

    let server_config = envy::from_env::<ServerConfig>().unwrap_or_default();
    let kafka_config = envy::from_env::<KafkaConfig>().unwrap_or_default();
    let network_config = NetworkConfig::from_env();

    Config {
        server: server_config,
        kafka: kafka_config,
        network: network_config,
    }
}

pub async fn config() -> &'static Config {
    CONFIG.get_or_init(init_config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_server_config() {
        let server_config = ServerConfig::default();
        assert_eq!(server_config.host, "0.0.0.0");
        assert_eq!(server_config.port, 3000);
    }

    #[tokio::test]
    async fn test_default_kafka_config() {
        let kafka_config = KafkaConfig::default();
        assert_eq!(kafka_config.topic, "pragma-data");
    }

    #[tokio::test]
    async fn test_config_values() {
        std::env::set_var("RPC_URL", "http://my-super-cool-test-rpc");
        let config = init_config().await;
        assert_eq!(config.server_host(), "0.0.0.0");
        assert_eq!(config.server_port(), 3000);
        assert_eq!(config.kafka_topic(), "pragma-data");
    }

    // TODO(akhercha): Add tests for network config
}
