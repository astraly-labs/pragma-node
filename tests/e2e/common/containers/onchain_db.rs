use std::env::current_dir;

use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use crate::common::constants::DEFAULT_PG_PORT;

use super::utils::run_migrations;
use super::Timescale;

#[rstest::fixture]
pub async fn setup_onchain_db() -> ContainerAsync<Timescale> {
    Postgres::default()
        .with_name("timescale/timescaledb-ha")
        .with_tag("pg14-latest")
        .with_env_var("POSTGRES_DB", "pragma")
        .with_env_var("POSTGRES_PASSWORD", "test-password")
        .with_mapped_port(5433, DEFAULT_PG_PORT.tcp())
        .with_env_var("TIMESCALEDB_TELEMETRY", "off")
        // .with_env_var("PGPORT", "5433")
        .with_network("pragma-tests-network")
        .with_container_name("test-onchain-db")
        .start()
        .await
        .unwrap()
}

pub async fn run_onchain_migrations(port: u16) {
    let db_url = format!(
        "postgres://postgres:test-password@localhost:{}/pragma",
        port
    );
    let migrations_folder = current_dir()
        .unwrap()
        .join("..")
        .join("infra")
        .join("pragma-node")
        .join("postgres_migrations");

    tracing::info!("{:?}", migrations_folder);
    run_migrations(&db_url, migrations_folder).await;
}
