use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::error::PragmaConsumerError;

lazy_static! {
    #[derive(Debug)]
    pub static ref CONFIG: Ingestor = load_configuration();
}

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
