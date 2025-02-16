use std::{borrow::Cow, collections::HashMap, env::current_dir, path::PathBuf, time::Duration};

use testcontainers::{
    core::{wait::HttpWaitStrategy, ContainerPort, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, Image, ImageExt,
};

use super::{
    offchain_db::OFFCHAIN_DB_CONTAINER_NAME, onchain_db::ONCHAIN_DB_CONTAINER_NAME,
    utils::image_builder::ImageBuilder,
};

const PRAGMA_NODE_BUILD_NAME: &str = "pragma-node-e2e";
const TAG: &str = "latest";

const PRAGMA_NODE_CONTAINER_NAME: &str = "pragma-node-container";

// Main port of the API
pub const SERVER_PORT: u16 = 3000;

// Port where we expose pragma-node metrics
const METRICS_PORT: u16 = 8080;

// Port used by both databases in their container
const DB_PORT: u16 = 5432;

#[rstest::fixture]
pub async fn setup_pragma_node() -> ContainerAsync<PragmaNode> {
    // 1. Build the pragma-node image
    ImageBuilder::default()
        .with_build_name(PRAGMA_NODE_BUILD_NAME)
        .with_dockerfile(&pragma_node_dockerfile_path())
        .build()
        .await;

    // 2. Run the container
    PragmaNode::default()
        .with_offchain_url(&db_connection_url(OFFCHAIN_DB_CONTAINER_NAME))
        .with_onchain_url(&db_connection_url(ONCHAIN_DB_CONTAINER_NAME))
        // We run as mode "dev" even though it's production, so we don't build the PragmaSigner
        // for now.
        .with_mode("dev")
        .with_mapped_port(SERVER_PORT, SERVER_PORT.tcp())
        .with_mapped_port(METRICS_PORT, METRICS_PORT.tcp())
        .with_network("pragma-tests-network")
        .with_container_name(PRAGMA_NODE_CONTAINER_NAME)
        .with_startup_timeout(Duration::from_secs(600))
        .start()
        .await
        .unwrap()
}

#[derive(Debug, Clone)]
pub struct PragmaNode {
    env_vars: HashMap<String, String>,
}

impl PragmaNode {
    /// Sets the database max connections. Defaults to 25.
    pub fn with_max_conn(mut self, conns: &str) -> Self {
        self.env_vars
            .insert("DATABASE_MAX_CONN".to_owned(), conns.to_owned());
        self
    }

    /// Sets the API port. Defaults to 3000.
    pub fn with_port(mut self, port: &str) -> Self {
        self.env_vars.insert("PORT".to_owned(), port.to_owned());
        self
    }

    /// Sets the metrics port. Defaults to 8080.
    pub fn with_metrics_port(mut self, port: &str) -> Self {
        self.env_vars
            .insert("METRICS_PORT".to_owned(), port.to_owned());
        self
    }

    /// Sets the application mode. Defaults to dev.
    pub fn with_mode(mut self, mode: &str) -> Self {
        self.env_vars.insert("MODE".to_owned(), mode.to_owned());
        self
    }

    /// Sets the offchain database URL.
    pub fn with_offchain_url(mut self, db_url: &str) -> Self {
        self.env_vars
            .insert("OFFCHAIN_DATABASE_URL".to_owned(), db_url.to_owned());
        self
    }

    /// Sets the onchain database URL.
    pub fn with_onchain_url(mut self, db_url: &str) -> Self {
        self.env_vars
            .insert("ONCHAIN_DATABASE_URL".to_owned(), db_url.to_owned());
        self
    }
}

impl Image for PragmaNode {
    fn name(&self) -> &str {
        PRAGMA_NODE_BUILD_NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::http(
            HttpWaitStrategy::new("/node")
                .with_port(ContainerPort::Tcp(SERVER_PORT))
                .with_expected_status_code(200_u16),
        )]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }
}

impl Default for PragmaNode {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("DATABASE_MAX_CONN".to_owned(), "25".to_owned());
        env_vars.insert("TOPIC".to_owned(), "pragma-data".to_owned());
        env_vars.insert("KAFKA_BROKERS".to_owned(), "pragma-data".to_owned());
        env_vars.insert("PORT".to_owned(), "3000".to_owned());
        env_vars.insert("METRICS_PORT".to_owned(), "8080".to_owned());

        Self { env_vars }
    }
}

// Utilities for build

// Returns the path of the Pragma node dockerfile.
fn pragma_node_dockerfile_path() -> PathBuf {
    current_dir()
        .unwrap()
        .join("..")
        .join("infra")
        .join("pragma-node")
        .join("Dockerfile")
}

// Builds a connection URL from an host & db port.
fn db_connection_url(host: &str) -> String {
    format!(
        "postgres://postgres:test-password@{}:{}/pragma",
        host, DB_PORT
    )
}
