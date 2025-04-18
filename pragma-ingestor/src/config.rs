use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::error::PragmaConsumerError;

pub static CONFIG: LazyLock<Ingestor> = LazyLock::new(load_configuration);

#[derive(Debug, Serialize, Deserialize)]
pub struct Ingestor {
    pub num_consumers: usize,
}

impl Ingestor {
    pub fn from_env() -> Result<Self, PragmaConsumerError> {
        envy::from_env::<Self>().map_err(PragmaConsumerError::LoadConfig)
    }
}

pub fn load_configuration() -> Ingestor {
    Ingestor::from_env().expect("cannot load configuration env")
}
