use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use crate::common::constants::DEFAULT_PG_PORT;

use super::Timescale;

pub const OFFCHAIN_DB_CONTAINER_NAME: &str = "test-offchain-db";

#[rstest::fixture]
pub async fn setup_offchain_db() -> ContainerAsync<Timescale> {
    Postgres::default()
        .with_name("timescale/timescaledb-ha")
        .with_tag("pg14-latest")
        .with_env_var("POSTGRES_DB", "pragma")
        .with_env_var("POSTGRES_PASSWORD", "test-password")
        .with_env_var("TIMESCALEDB_TELEMETRY", "off")
        .with_env_var("PGPORT", "5435")
        .with_mapped_port(5435, DEFAULT_PG_PORT.tcp())
        .with_network("pragma-tests-db-network")
        .with_container_name(OFFCHAIN_DB_CONTAINER_NAME)
        .start()
        .await
        .unwrap()
}
