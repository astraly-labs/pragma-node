use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::error::ErrorKind;

lazy_static! {
    #[derive(Debug)]
    pub static ref CONFIG: Ingestor = load_configuration();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ingestor {
    pub brokers: Vec<String>,
    pub topic: String,
    pub group_id: String,
}

impl Ingestor {
    pub fn from_env() -> Result<Self, ErrorKind> {
        envy::from_env::<Ingestor>()
            .map_err(ErrorKind::LoadConfig)
    }
}

pub fn load_configuration() -> Ingestor {
    Ingestor::from_env().expect("cannot load configuration env")
}
