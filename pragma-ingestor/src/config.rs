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
        envy::from_env::<Self>().map_err(ErrorKind::LoadConfig)
    }
}

pub fn load_configuration() -> Ingestor {
    Ingestor::from_env().expect("cannot load configuration env")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_ingestor_init() {
        let brokers = vec!["localhost:9092".to_string()];
        let ingestor = Ingestor {
            brokers: brokers.clone(),
            topic: "test_topic".to_string(),
            group_id: "test_group".to_string(),
        };

        assert_eq!(ingestor.brokers, brokers);
        assert_eq!(ingestor.topic, "test_topic");
        assert_eq!(ingestor.group_id, "test_group");
    }

    #[test]
    fn test_load_from_env() {
        unsafe {
            env::set_var("BROKERS", "localhost:9092");
            env::set_var("TOPIC", "test_topic");
            env::set_var("GROUP_ID", "test_group");
        }

        let ingestor = Ingestor::from_env().unwrap();

        assert_eq!(ingestor.brokers, vec!["localhost:9092".to_string()]);
        assert_eq!(ingestor.topic, "test_topic");
        assert_eq!(ingestor.group_id, "test_group");
        unsafe {
            env::remove_var("BROKERS");
            env::remove_var("TOPIC");
            env::remove_var("GROUP_ID");
        }
    }

    #[test]
    fn test_env_error_handling() {
        unsafe {
            env::remove_var("BROKERS");
            env::remove_var("TOPIC");
            env::remove_var("GROUP_ID");
        }

        let result = Ingestor::from_env();
        assert!(result.is_err());
    }
}
