pub mod kafka;
pub mod offchain_db;
pub mod onchain_db;
pub mod pragma_node;
pub mod utils;
pub mod zookeeper;

use std::sync::Arc;

use pragma_node::PragmaNode;
use testcontainers::ContainerAsync;
use testcontainers_modules::kafka::Kafka;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::zookeeper::Zookeeper;

use crate::common::containers::onchain_db::run_onchain_migrations;
use crate::common::containers::{
    kafka::setup_kafka, offchain_db::setup_offchain_db, onchain_db::setup_onchain_db,
    pragma_node::setup_pragma_node, zookeeper::setup_zookeeper,
};
use crate::common::logs::init_logging;

// Postgres from testcontainers-modules works the same as Timescale.
// Instead of creating a whole new Image we just use this one but rename it
// timescale in our test suite.
pub type Timescale = Postgres;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Containers {
    pub offchain_db: Arc<ContainerAsync<Timescale>>,
    pub onchain_db: Arc<ContainerAsync<Timescale>>,
    pub zookeeper: Arc<ContainerAsync<Zookeeper>>,
    pub kafka: Arc<ContainerAsync<Kafka>>,
    pub pragma_node: Arc<ContainerAsync<PragmaNode>>,
}

#[rstest::fixture]
pub async fn setup_containers(
    #[from(init_logging)] _logging: (),
    #[future] setup_offchain_db: ContainerAsync<Timescale>,
    #[future] setup_onchain_db: ContainerAsync<Timescale>,
    #[future] setup_zookeeper: ContainerAsync<Zookeeper>,
    #[future] setup_kafka: ContainerAsync<Kafka>,
    #[future] setup_pragma_node: ContainerAsync<PragmaNode>,
) -> Containers {
    tracing::info!("🔨 Setup offchain db..");
    let offchain_db = setup_offchain_db.await;
    tracing::info!("✅ ... offchain db ready!\n");

    tracing::info!("🔨 Setup onchain db..");
    let onchain_db = setup_onchain_db.await;
    let onchain_db_port: u16 = onchain_db.get_host_port_ipv4(5432).await.unwrap();
    tracing::info!("🪛 Executing onchain migrations...");
    run_onchain_migrations(onchain_db_port).await;
    tracing::info!("✅ ... onchain db ready!\n");

    tracing::info!("🔨 Setup zookeeper..");
    let zookeeper = setup_zookeeper.await;
    tracing::info!("✅ ... zookeeper!\n");

    tracing::info!("🔨 Setup kafka..");
    let kafka = setup_kafka.await;
    tracing::info!("✅ ... kafka!\n");

    tracing::info!("🔨 Setup pragma_node...");
    let pragma_node = setup_pragma_node.await;
    tracing::info!("✅ ... pragma-node!\n");

    Containers {
        onchain_db: Arc::new(onchain_db),
        offchain_db: Arc::new(offchain_db),
        zookeeper: Arc::new(zookeeper),
        kafka: Arc::new(kafka),
        pragma_node: Arc::new(pragma_node),
    }
}
