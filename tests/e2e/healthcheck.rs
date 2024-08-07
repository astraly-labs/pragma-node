use pretty_assertions::assert_eq;
use rstest::rstest;
use testcontainers::ContainerAsync;

use crate::common::constants::{DEFAULT_PG_PORT, PRAGMA_NODE_CONTAINER_NAME};
use crate::common::containers::{
    offchain_db::setup_offchain_db, onchain_db::setup_onchain_db, pragma_node::setup_pragma_node,
    utils::kill_and_remove_container, Timescale,
};
#[rstest]
#[tokio::test]
#[tracing_test::traced_test]
async fn healthcheck_ok(
    #[future] setup_offchain_db: ContainerAsync<Timescale>,
    #[future] setup_onchain_db: ContainerAsync<Timescale>,
) {
    tracing::info!("🔨 Setup offchain db..");
    let offchain_db = setup_offchain_db.await;
    let offchain_db_port: u16 = offchain_db
        .get_host_port_ipv4(DEFAULT_PG_PORT)
        .await
        .unwrap();
    tracing::info!("✅ ... offchain db!");

    tracing::info!("🔨 Setup onchain db..");
    let onchain_db = setup_onchain_db.await;
    let onchain_db_port: u16 = onchain_db
        .get_host_port_ipv4(DEFAULT_PG_PORT)
        .await
        .unwrap();
    tracing::info!("✅ ... onchain db!");

    tracing::info!("🔨 Setup pragma_node...");
    setup_pragma_node(offchain_db_port, onchain_db_port).await;
    tracing::info!("✅ ... pragma-node!");

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
