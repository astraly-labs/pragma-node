use std::fs;
use std::path::PathBuf;

use deadpool_diesel::postgres::{Manager, Pool};
use diesel::connection::SimpleConnection;
use tokio::process::Command;

pub async fn kill_and_remove_container(container_name: &str) {
    let _ = Command::new("docker")
        .args(["kill", container_name])
        .output()
        .await;

    let cmd = Command::new("docker")
        .args(["rm", container_name])
        .output()
        .await
        .unwrap_or_else(|_| panic!("Failed to teardown container {}", container_name));

    if !cmd.status.success() {
        panic!(
            "Failed to kill pragma-node container: {}",
            String::from_utf8_lossy(&cmd.stderr)
        );
    }
}

pub async fn run_migrations(database_url: &str, folder: PathBuf) {
    // Create a connection pool
    let manager = Manager::new(database_url.to_string(), deadpool_diesel::Runtime::Tokio1);
    let pool = Pool::builder(manager).build().unwrap();

    // Read and sort migration files
    let mut migration_files = read_migration_files(folder);
    migration_files.sort_by(|a, b| a.0.cmp(&b.0));

    // Execute migrations sequentially
    for (_, file_path) in migration_files {
        execute_migration(&pool, file_path).await;
    }
}

fn read_migration_files(folder: PathBuf) -> Vec<(u32, PathBuf)> {
    fs::read_dir(folder)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension()? != "sql" {
                return None;
            }
            let file_name = path.file_name()?.to_str()?;
            let (prefix, _) = file_name.split_once('-')?;
            let number = prefix.parse::<u32>().ok()?;
            Some((number, path))
        })
        .collect()
}

async fn execute_migration(pool: &Pool, file_path: PathBuf) {
    let sql = fs::read_to_string(&file_path).unwrap();
    let conn = pool.get().await.unwrap();

    conn.interact(move |conn| conn.batch_execute(&sql))
        .await
        .unwrap()
        .unwrap();
}
