use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ErrorKind;

lazy_static! {
    #[derive(Debug)]
    pub static ref CONFIG: Ingestor = load_configuration();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ingestor {
    pub kafka: Kafka,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Kafka {
    pub brokers: Vec<String>,
    pub topic: String,
    pub group_id: String,
}

impl Ingestor {
    pub fn from_file(config_file: &str) -> Result<Self, ErrorKind> {
        toml::from_str(
            std::fs::read_to_string(config_file)
                .map_err(ErrorKind::ReadConfig)?
                .as_str(),
        )
            .map_err(ErrorKind::LoadConfig)
    }
}

pub fn load_configuration() -> Ingestor {
    let path = std::env::var("INGESTOR_CONF")
        .expect("can't find configuration path.");
    info!("loading configuration file '{}'", path);
    Ingestor::from_file(path.as_str())
        .expect("can't load configuration file.")
}