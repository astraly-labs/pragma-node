use std::process::Command as SyncCommand;
use std::{env::current_dir, path::PathBuf, time::Duration};

use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::time::sleep;

use super::offchain_db::OFFCHAIN_DB_CONTAINER_NAME;
use super::onchain_db::ONCHAIN_DB_CONTAINER_NAME;

pub const PRAGMA_NODE_CONTAINER_NAME: &str = "pragma-node-container";

#[derive(Debug, Clone)]
pub struct PragmaNodeContainer {
    offchain_port: u16,
    onchain_port: u16,
}

impl PragmaNodeContainer {
    pub async fn new(offchain_port: u16, onchain_port: u16) -> Self {
        let container = Self {
            offchain_port,
            onchain_port,
        };
        container.setup().await;
        container
    }

    pub fn base_url(&self) -> &str {
        "http://localhost:3000"
    }

    pub fn container_name(&self) -> &str {
        PRAGMA_NODE_CONTAINER_NAME
    }

    async fn setup(&self) {
        let dockerfile_path = self.dockerfile_path();

        // Build the pragma-node Docker image
        let output = Command::new("docker")
            .args([
                "buildx",
                "build",
                "--file",
                dockerfile_path.to_str().unwrap(),
                "--force-rm",
                "--tag",
                "pragma-node-e2e",
                "..",
            ])
            .output()
            .await
            .expect("Failed to execute Docker build command");

        if !output.status.success() {
            tracing::error!("{}", String::from_utf8(output.stderr).unwrap());
            panic!("Failed to build pragma-node");
        }

        // Run the pragma-node Docker container
        let output = Command::new("docker")
            .args([
                "run",
                "-d",
                "--name",
                self.container_name(),
                "--network",
                "pragma-tests-db-network",
                "--network",
                "pragma-tests-kafka-network",
                "-p",
                "3000:3000",
                "-p",
                "8080:8080",
                "-e",
                "DATABASE_MAX_CONN=25",
                "-e",
                "TOPIC=pragma-data",
                "-e",
                "KAFKA_BROKERS=pragma-data",
                "-e",
                &format!(
                    "OFFCHAIN_DATABASE_URL={}",
                    self.db_connection_url(OFFCHAIN_DB_CONTAINER_NAME, self.offchain_port)
                ),
                "-e",
                &format!(
                    "ONCHAIN_DATABASE_URL={}",
                    self.db_connection_url(ONCHAIN_DB_CONTAINER_NAME, self.onchain_port)
                ),
                "-e",
                "METRICS_PORT=8080",
                "pragma-node-e2e",
            ])
            .output()
            .await
            .expect("Failed to run Docker container");

        if !output.status.success() {
            tracing::error!("{}", String::from_utf8(output.stderr).unwrap());
            panic!("Failed to run pragma-node");
        }

        self.wait_is_healthy().await;
    }

    async fn wait_is_healthy(&self) {
        let max_retries = 20;
        let retry_interval = Duration::from_secs(5);

        for attempt in 1..=max_retries {
            match TcpStream::connect("localhost:3000").await {
                Ok(_) => {
                    tracing::info!("🪛 Applying pragma-node migrations...");
                    sleep(Duration::from_secs(15)).await;
                    return;
                }
                _ => {
                    if attempt == max_retries {
                        panic!("pragma-node failed to start after {} attempts", max_retries);
                    }
                    sleep(retry_interval).await;
                }
            }
        }
    }

    fn db_connection_url(&self, host: &str, db_port: u16) -> String {
        format!(
            "postgres://postgres:test-password@{}:{}/pragma",
            host, db_port
        )
    }

    fn dockerfile_path(&self) -> PathBuf {
        let mut current_dir = current_dir().unwrap();
        if current_dir.ends_with("tests") {
            current_dir = current_dir.join("..");
        }
        current_dir
            .join("infra")
            .join("pragma-node")
            .join("Dockerfile")
    }
}

impl Drop for PragmaNodeContainer {
    fn drop(&mut self) {
        let _ = SyncCommand::new("docker")
            .args(["kill", self.container_name()])
            .output();

        let cmd = SyncCommand::new("docker")
            .args(["rm", self.container_name()])
            .output()
            .unwrap_or_else(|_| panic!("Failed to teardown container {}", self.container_name()));

        if !cmd.status.success() {
            eprintln!(
                "Failed to remove pragma-node container: {}",
                String::from_utf8_lossy(&cmd.stderr)
            );
        }
    }
}
