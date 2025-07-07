use std::env;

use pragma_entities::connection::{init_pool, ENV_OFFCHAIN_DATABASE_URL};
use pragma_entities::db::run_migrations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    println!("Running database migrations...");

    // Initialize the database pool
    let offchain_pool = init_pool("pragma-migrations", ENV_OFFCHAIN_DATABASE_URL)
        .expect("Failed to initialize offchain database pool");

    // Run the migrations
    run_migrations(&offchain_pool).await;

    println!("Database migrations completed successfully!");

    Ok(())
} 