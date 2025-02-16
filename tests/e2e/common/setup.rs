use std::sync::Arc;

use deadpool_diesel::{postgres::Pool, Manager};
use diesel::RunQueryDsl;

use testcontainers::ContainerAsync;
use testcontainers_modules::kafka::Kafka;
use testcontainers_modules::zookeeper::Zookeeper;

use crate::common::containers::{
    kafka::{init_kafka_topics, setup_kafka},
    offchain_db::setup_offchain_db,
    onchain_db::{run_onchain_migrations, setup_onchain_db},
    pragma_node::{setup_pragma_node, PragmaNode, SERVER_PORT},
    zookeeper::setup_zookeeper,
    Containers, Timescale,
};
use crate::common::logs::init_logging;

/// Main structure that we carry around for our tests.
/// Contains some usefull fields & functions attached to make testing easier.
pub struct TestHelper {
    pub node_base_url: String,
    pub onchain_pool: Pool,
    pub offchain_pool: Pool,
    pub containers: Containers,
}

impl TestHelper {
    pub fn endpoint(&self, path: &str) -> String {
        format!("{}/{}", self.node_base_url, path)
    }

    /// Executes the provided `sql` query on the database `Pool`.
    pub async fn execute_sql(&self, pool: &Pool, sql: String) {
        let conn = pool
            .get()
            .await
            .expect("Failed to get connection from pool");

        let sql = sql.to_owned();
        conn.interact(move |conn| diesel::sql_query(sql).execute(conn))
            .await
            .expect("Failed to execute interact closure")
            .expect("Failed to execute SQL query");
    }
}

/// Setup all the containers needed for integration tests and return a
/// `TestHelper` structure containing handles to interact with
/// the containers.
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
    tracing::info!("✅ ... zookeeper ready!\n");

    tracing::info!("🔨 Setup kafka..");
    let kafka = setup_kafka.await;
    init_kafka_topics(&kafka).await;
    tracing::info!("✅ ... kafka ready!\n");

    tracing::info!("🔨 Setup pragma_node...");
    let pragma_node = setup_pragma_node.await;
    tracing::info!("✅ ... pragma-node ready!\n");

    let containers = Containers {
        onchain_db: Arc::new(onchain_db),
        offchain_db: Arc::new(offchain_db),
        zookeeper: Arc::new(zookeeper),
        kafka: Arc::new(kafka),
        pragma_node: Arc::new(pragma_node),
    };

    TestHelper {
        node_base_url: format!("http://localhost:{}", SERVER_PORT),
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
