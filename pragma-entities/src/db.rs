use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::sql_query;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations/");

pub async fn run_migrations(pool: &Pool) {
    let conn = pool.get().await.expect("Failed to get DB connection");

    // First ensure TimescaleDB and its toolkit extension are enabled
    conn.interact(|conn| {
        sql_query("CREATE EXTENSION IF NOT EXISTS timescaledb;").execute(conn)?;
        sql_query("CREATE EXTENSION IF NOT EXISTS timescaledb_toolkit;").execute(conn)
    })
    .await
    .expect("Failed to enable TimescaleDB extensions")
    .expect("Database error while enabling TimescaleDB extensions");

    // Then run the migrations
    conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
        .await
        .expect("Failed to run migrations")
        .expect("Database error during migration");
}
