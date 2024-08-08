use std::sync::Arc;

use testcontainers::ContainerAsync;
use testcontainers_modules::kafka::Kafka;
use testcontainers_modules::zookeeper::Zookeeper;

use crate::common::containers::{
    kafka::setup_kafka,
    offchain_db::setup_offchain_db,
    onchain_db::setup_onchain_db,
    pragma_node::{setup_pragma_node, PragmaNode},
    zookeeper::setup_zookeeper,
    Containers, Timescale,
};
use crate::common::logs::init_logging;

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
