use std::env::current_dir;

use deadpool_diesel::postgres::Pool;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use super::utils::migrations::run_migrations;
use super::Timescale;

pub const ONCHAIN_DB_CONTAINER_NAME: &str = "test-onchain-db";

#[rstest::fixture]
pub async fn setup_onchain_db() -> ContainerAsync<Timescale> {
    Postgres::default()
        .with_name("timescale/timescaledb-ha")
        .with_tag("pg14-latest")
        .with_env_var("POSTGRES_DB", "pragma")
        .with_env_var("POSTGRES_PASSWORD", "test-password")
        .with_env_var("TIMESCALEDB_TELEMETRY", "off")
        .with_network("pragma-tests-network")
        .with_container_name(ONCHAIN_DB_CONTAINER_NAME)
        .start()
        .await
        .unwrap()
}

pub async fn run_onchain_migrations(db_pool: &Pool) {
    let migrations_folder = current_dir()
        .unwrap()
        .join("..")
        .join("infra")
        .join("pragma-node")
        .join("postgres_migrations");

    run_migrations(db_pool, migrations_folder).await;
}
