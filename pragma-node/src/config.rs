use dotenvy::dotenv;
use serde::Deserialize;
use tokio::sync::OnceCell;

#[derive(Debug, Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Deserialize)]
struct KafkaConfig {
    topic: String,
}

#[derive(Debug)]
pub struct Config {
    server: ServerConfig,
    kafka: KafkaConfig,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            topic: "pragma-data".to_string(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
        }
    }
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
}

pub static CONFIG: OnceCell<Config> = OnceCell::const_new();

async fn init_config() -> Config {
    dotenv().ok();

    let server_config = envy::from_env::<ServerConfig>().unwrap_or_default();

    let kafka_config = envy::from_env::<KafkaConfig>().unwrap_or_default();

    Config {
        server: server_config,
        kafka: kafka_config,
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
        let config = init_config().await;
        assert_eq!(config.server_host(), "0.0.0.0");
        assert_eq!(config.server_port(), 3000);
        assert_eq!(config.kafka_topic(), "pragma-data");
    }
}
