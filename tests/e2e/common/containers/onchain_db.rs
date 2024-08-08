use std::env::current_dir;

use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use super::utils::migrations::run_migrations;
use super::Timescale;

pub const ONCHAIN_DB_CONTAINER_NAME: &str = "test-onchain-db";
const PORT: u16 = 5432;

#[rstest::fixture]
pub async fn setup_onchain_db() -> ContainerAsync<Timescale> {
    // 1. Run the container
    let onchain_container = Postgres::default()
        .with_name("timescale/timescaledb-ha")
        .with_tag("pg14-latest")
        .with_env_var("POSTGRES_DB", "pragma")
        .with_env_var("POSTGRES_PASSWORD", "test-password")
        .with_env_var("TIMESCALEDB_TELEMETRY", "off")
        .with_network("pragma-tests-network")
        .with_container_name(ONCHAIN_DB_CONTAINER_NAME)
        .start()
        .await
        .unwrap();

    // 2. Run the migrations
    let onchain_db_port: u16 = onchain_container.get_host_port_ipv4(PORT).await.unwrap();
    run_onchain_migrations(onchain_db_port).await;

    onchain_container
}

async fn run_onchain_migrations(port: u16) {
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

    run_migrations(&db_url, migrations_folder).await;
}
