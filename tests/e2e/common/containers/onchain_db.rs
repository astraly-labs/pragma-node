use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use super::Timescale;

#[rstest::fixture]
pub async fn create_onchain_db() -> ContainerAsync<Timescale> {
    Postgres::default()
        .with_name("timescale/timescaledb-ha")
        .with_tag("pg14-latest")
        .with_env_var("POSTGRES_DB", "pragma")
        .with_env_var("REDIS_PASSWORD", "test-password")
        .with_env_var("PGPORT", "5433")
        .start()
        .await
        .unwrap()
}
