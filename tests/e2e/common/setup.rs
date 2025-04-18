use std::sync::Arc;

use deadpool_diesel::{Manager, postgres::Pool};
use diesel::RunQueryDsl;

use pragma_common::{AggregationMode, Interval, InstrumentType};
use pragma_node::utils::sql::{get_interval_specifier, get_table_suffix};
use testcontainers::ContainerAsync;
use testcontainers_modules::kafka::Kafka;
use testcontainers_modules::zookeeper::Zookeeper;

use crate::common::containers::{
    Containers, Timescale,
    kafka::{init_kafka_topics, setup_kafka},
    offchain_db::setup_offchain_db,
    onchain_db::{run_onchain_migrations, setup_onchain_db},
    pragma_node::{
        SERVER_PORT,
        docker::{PragmaNode, setup_pragma_node_with_docker},
        local::setup_pragma_node_with_cargo,
    },
    zookeeper::setup_zookeeper,
};
use crate::common::logs::init_logging;

use super::{containers::pragma_node::PragmaNodeMode, utils::get_window_size};

/// Main structure that we carry around for our tests.
/// Contains some usefull fields & functions attached to make testing easier.
pub struct TestHelper {
    pub node_base_url: String,
    pub onchain_pool: Pool,
    pub offchain_pool: Pool,
    pub containers: Containers,
    pub pragma_node_mode: PragmaNodeMode,
    pub node_handle: Option<tokio::process::Child>,
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

        conn.interact(move |conn| diesel::sql_query(sql).execute(conn))
            .await
            .expect("Failed to execute interact closure")
            .expect("Failed to execute SQL query");
    }

    pub async fn execute_sql_many(&self, pool: &Pool, sql_many: Vec<String>) {
        for sql in sql_many {
            self.execute_sql(pool, sql).await;
        }
    }

    /// Refreshes a TimescaleDB continuous aggregate materialized view around a specific timestamp.
    /// The refreshed view will be automatically found depending on the interval + aggregation mode.
    /// NOTE: It does not work with future entries for now since we don't care for our tests yet.
    pub async fn refresh_offchain_continuous_aggregate(
        &self,
        timestamp: u64,
        interval: Interval,
        aggregation: AggregationMode,
    ) {
        let interval_spec =
            get_interval_specifier(interval, matches!(aggregation, AggregationMode::Twap)).unwrap();
        let window_size = get_window_size(interval);
        let suffix = get_table_suffix(InstrumentType::Spot).unwrap();

        let table_name = if matches!(aggregation, AggregationMode::Twap) {
            "twap"
        } else {
            "median"
        };

        let sql = format!(
            r"
            CALL refresh_continuous_aggregate(
                '{}_{}_{}',
                to_timestamp({} - {}),
                to_timestamp({} + {})
            );",
            table_name, interval_spec, suffix, timestamp, window_size, timestamp, window_size
        );

        self.execute_sql(&self.offchain_pool, sql).await;
    }

    /// Allows to shutdown a pragma-node local instance ran with cargo (not with docker).
    pub async fn shutdown_local_pragma_node(&mut self) {
        if matches!(self.pragma_node_mode, PragmaNodeMode::Docker) {
            return;
        }
        if let Some(mut handle) = self.node_handle.take() {
            handle
                .kill()
                .await
                .expect("Failed to kill pragma-node process");
        }
    }
}

// TODO: Very flaky. See if we can force the kill of the handle without async.
// TODO: At the moment, we need to call `shutdown_local_pragma_node` ourselves.
/// Automatically kills the local `pragma-node` instance when we quit the scope of a test.
impl Drop for TestHelper {
    fn drop(&mut self) {
        if let Some(handle) = &mut self.node_handle {
            handle
                .start_kill()
                .expect("Failed to send kill signal to pragma-node");
        }
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
    #[future] setup_pragma_node_with_docker: ContainerAsync<PragmaNode>,
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

    // NOTE: See the `Default` impl. Already set depending on the `PRAGMA_NODE_MODE` env var.
    let pragma_node_mode = PragmaNodeMode::default();

    let (pragma_node, node_handle) = match pragma_node_mode {
        PragmaNodeMode::Docker => {
            tracing::info!("🔨 Setup pragma_node in Docker mode...");
            let node = setup_pragma_node_with_docker.await;
            (Some(Arc::new(node)), None)
        }
        PragmaNodeMode::Local => {
            tracing::info!("🔨 Starting pragma_node in local mode...");
            let offchain_db_port: u16 = offchain_db.get_host_port_ipv4(5432).await.unwrap();
            let onchain_db_port: u16 = onchain_db.get_host_port_ipv4(5432).await.unwrap();
            let handle = setup_pragma_node_with_cargo(offchain_db_port, onchain_db_port).await;
            (None, Some(handle))
        }
    };

    tracing::info!("✅ ... pragma-node ready!\n");

    let containers = Containers {
        onchain_db: Arc::new(onchain_db),
        offchain_db: Arc::new(offchain_db),
        zookeeper: Arc::new(zookeeper),
        kafka: Arc::new(kafka),
        pragma_node,
    };

    TestHelper {
        node_base_url: format!("http://localhost:{}", SERVER_PORT),
        containers,
        onchain_pool,
        offchain_pool,
        pragma_node_mode,
        node_handle,
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
