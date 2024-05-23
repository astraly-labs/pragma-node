use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct KafkaConfig {
    pub topic: String,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            topic: "pragma-data".to_string(),
        }
    }
}
