use pretty_assertions::assert_eq;
use rstest::rstest;
use testcontainers::ContainerAsync;
use tokio::time::{sleep, Duration};

use crate::common::containers::{
    offchain_db::setup_offchain_db, onchain_db::setup_onchain_db, pragma_node::setup_pragma_node,
    Timescale,
};
use crate::common::logs::init_logging;

#[rstest]
#[tokio::test]
async fn healthcheck_ok(
    #[from(init_logging)] _logging: (),
    #[future] setup_offchain_db: ContainerAsync<Timescale>,
    #[future] setup_onchain_db: ContainerAsync<Timescale>,
) {
    tracing::info!("🔨 Setup offchain db..");
    let offchain_db = setup_offchain_db.await;
    let host_ip = offchain_db.get_host().await.unwrap();
    assert_eq!(host_ip.to_string(), "localhost");
    let offchain_db_port: u16 = offchain_db.get_host_port_ipv4(5432).await.unwrap();
    tracing::info!("expose ports: {:?}", offchain_db.ports().await.unwrap());
    tracing::info!("offchain port: {}", offchain_db_port);
    tracing::info!("✅ offchain db!");

    tracing::info!("🔨 Setup onchain db..");
    let onchain_db = setup_onchain_db.await;
    let host_ip = onchain_db.get_host().await.unwrap();
    assert_eq!(host_ip.to_string(), "localhost");
    let onchain_db_port: u16 = onchain_db.get_host_port_ipv4(5432).await.unwrap();
    tracing::info!("expose ports: {:?}", offchain_db.ports().await.unwrap());
    tracing::info!("onchain port: {}", onchain_db_port);
    tracing::info!("✅ onchain db!");

    tracing::info!("🔨 Setup pragma_node...");
    setup_pragma_node(offchain_db_port, onchain_db_port);
    tracing::info!("✅ pragma-node!");

    sleep(Duration::from_secs(10)).await;

    let body = reqwest::get("http://localhost:3000/node")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(body.trim(), "Server is running!");
}
