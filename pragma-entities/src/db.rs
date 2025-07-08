use deadpool_diesel::postgres::Pool;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations/");

pub async fn run_migrations(pool: &Pool) {
    let conn = pool.get().await.expect("Failed to get DB connection");

    // Then run the migrations
    conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
        .await
        .expect("Failed to run migrations")
        .expect("Database error during migration");
}
