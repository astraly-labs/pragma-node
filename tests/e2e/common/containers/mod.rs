pub mod kafka;
pub mod offchain_db;
pub mod onchain_db;
pub mod pragma_node;
pub mod utils;
pub mod zookeeper;

use std::sync::Arc;

use pragma_node::PragmaNode;
use testcontainers::ContainerAsync;
use testcontainers_modules::{kafka::Kafka, postgres::Postgres, zookeeper::Zookeeper};

// Postgres from testcontainers-modules works the same as Timescale.
// Instead of creating a whole new Image we just use this one but rename it
// timescale in our test suite.
pub type Timescale = Postgres;

#[derive(Debug)]
pub struct Containers {
    pub offchain_db: Arc<ContainerAsync<Timescale>>,
    pub onchain_db: Arc<ContainerAsync<Timescale>>,
    pub zookeeper: Arc<ContainerAsync<Zookeeper>>,
    pub kafka: Arc<ContainerAsync<Kafka>>,
    pub pragma_node: Arc<ContainerAsync<PragmaNode>>,
}
