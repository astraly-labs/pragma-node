use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use super::Timescale;

#[rstest::fixture]
pub async fn setup_offchain_db() -> ContainerAsync<Timescale> {
    Postgres::default()
        .with_name("timescale/timescaledb-ha")
        .with_tag("pg14-latest")
        .with_env_var("POSTGRES_DB", "pragma")
        .with_env_var("POSTGRES_PASSWORD", "test-password")
        .with_mapped_port(5434, 5432_u16.tcp())
        .with_network("pragma-tests-network")
        .with_container_name("test-offchain-db")
        .start()
        .await
        .unwrap()
}
