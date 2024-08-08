use std::sync::Arc;

use deadpool_diesel::{postgres::Pool, Manager};
use testcontainers::ContainerAsync;
use testcontainers_modules::kafka::Kafka;
use testcontainers_modules::zookeeper::Zookeeper;

use crate::common::containers::{
    kafka::setup_kafka,
    offchain_db::setup_offchain_db,
    onchain_db::{run_onchain_migrations, setup_onchain_db},
    pragma_node::{setup_pragma_node, PragmaNode},
    zookeeper::setup_zookeeper,
    Containers, Timescale,
};
use crate::common::logs::init_logging;

/// Main structure that we carry around for our tests.
/// Contains some usefull fields & functions attached to make testing easier.
#[allow(dead_code)]
pub struct TestHelper {
    node_base_url: String,
    onchain_pool: Pool,
    offchain_pool: Pool,
    containers: Containers,
}

impl TestHelper {
    pub fn endpoint(&self, path: &str) -> String {
        format!("{}/{}", self.node_base_url, path)
    }
}

#[rstest::fixture]
pub async fn setup_containers(
    #[from(init_logging)] _logging: (),
    #[future] setup_offchain_db: ContainerAsync<Timescale>,
    #[future] setup_onchain_db: ContainerAsync<Timescale>,
    #[future] setup_zookeeper: ContainerAsync<Zookeeper>,
    #[future] setup_kafka: ContainerAsync<Kafka>,
    #[future] setup_pragma_node: ContainerAsync<PragmaNode>,
) -> TestHelper {
    tracing::info!("🔨 Setup offchain db..");
    let offchain_db = setup_offchain_db.await;
    let offchain_pool = get_db_pool(offchain_db.get_host_port_ipv4(5432).await.unwrap());
    tracing::info!("✅ ... offchain db ready!\n");

    tracing::info!("🔨 Setup onchain db..");
    let onchain_db = setup_onchain_db.await;
    let onchain_pool = get_db_pool(onchain_db.get_host_port_ipv4(5432).await.unwrap());
    run_onchain_migrations(&onchain_pool).await;
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

    let containers = Containers {
        onchain_db: Arc::new(onchain_db),
        offchain_db: Arc::new(offchain_db),
        zookeeper: Arc::new(zookeeper),
        kafka: Arc::new(kafka),
        pragma_node: Arc::new(pragma_node),
    };

    TestHelper {
        node_base_url: "http://localhost:3000".to_owned(),
        containers,
        onchain_pool,
        offchain_pool,
    }
}

fn get_db_pool(db_port: u16) -> Pool {
    let db_url = format!(
        "postgres://postgres:test-password@localhost:{}/pragma",
        db_port
    );
    let manager = Manager::new(db_url, deadpool_diesel::Runtime::Tokio1);
    Pool::builder(manager).build().unwrap()
}
