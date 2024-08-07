use pretty_assertions::assert_eq;
use rstest::rstest;
use testcontainers::ContainerAsync;
use testcontainers_modules::kafka::Kafka;
use testcontainers_modules::zookeeper::Zookeeper;

use crate::common::constants::{DEFAULT_PG_PORT, PRAGMA_NODE_CONTAINER_NAME};
use crate::common::containers::onchain_db::run_onchain_migrations;
use crate::common::containers::{
    kafka::setup_kafka, offchain_db::setup_offchain_db, onchain_db::setup_onchain_db,
    pragma_node::setup_pragma_node, utils::kill_and_remove_container, zookeeper::setup_zookeeper,
    Timescale,
};
use crate::common::logs::init_logging;
#[rstest]
#[tokio::test]
async fn healthcheck_ok(
    #[from(init_logging)] _logging: (),
    #[future] setup_offchain_db: ContainerAsync<Timescale>,
    #[future] setup_onchain_db: ContainerAsync<Timescale>,
    #[future] setup_zookeeper: ContainerAsync<Zookeeper>,
    #[future] setup_kafka: ContainerAsync<Kafka>,
) {
    tracing::info!("🔨 Setup offchain db..");
    let offchain_db = setup_offchain_db.await;
    let offchain_db_port: u16 = offchain_db
        .get_host_port_ipv4(DEFAULT_PG_PORT)
        .await
        .unwrap();
    tracing::info!("✅ ... offchain db ready (port={offchain_db_port})!\n");

    tracing::info!("🔨 Setup onchain db..");
    let onchain_db = setup_onchain_db.await;
    let onchain_db_port: u16 = onchain_db
        .get_host_port_ipv4(DEFAULT_PG_PORT)
        .await
        .unwrap();
    tracing::info!("✅ ... onchain db ready (port={onchain_db_port})!");

    tracing::info!("🔨 Executing onchain migrations...");
    run_onchain_migrations(onchain_db_port).await;
    tracing::info!("✅ ... onchain migrations ok!\n");

    tracing::info!("🔨 Setup zookeeper..");
    let _zookeeper = setup_zookeeper.await;
    tracing::info!("✅ ... zookeeper!\n");

    tracing::info!("🔨 Setup kafka..");
    let _kafka = setup_kafka.await;
    tracing::info!("✅ ... kafka!\n");

    tracing::info!("🔨 Setup pragma_node...");
    setup_pragma_node(offchain_db_port, onchain_db_port).await;
    tracing::info!("✅ ... pragma-node!\n");

    let body = reqwest::get("http://localhost:3000/node")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert_eq!(body.trim(), "Server is running!");

    // Teardown
    kill_and_remove_container(PRAGMA_NODE_CONTAINER_NAME).await;
}
