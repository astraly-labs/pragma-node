use std::{env::current_dir, path::PathBuf, time::Duration};
use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::time::sleep;

use crate::common::constants::PRAGMA_NODE_CONTAINER_NAME;

// TODO: implement a struct that start a container with some options + handle Drop
// It is not possible yet to start local Dockefiles using the testcontainers
// rust crate.
// To bypass this, we start it using the `docker` command ourselves.
pub async fn setup_pragma_node(offchain_port: u16, onchain_port: u16) {
    let dockerfile_path = pragma_node_dockerfile_path();

    // Build the pragma-node Docker images
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
        tracing::error!("Unable to build pragma-node-e2e");
        tracing::error!("{}", String::from_utf8(output.stderr).unwrap());
        panic!("Nope :)");
    }

    // Run the pragma-node Docker container with environment variables
    let output = Command::new("docker")
        .args([
            "run",
            "-d", // Run in detached mode
            "--name",
            PRAGMA_NODE_CONTAINER_NAME,
            "--network",
            "pragma-tests-network",
            "-p",
            "3000:3000", // Node API port
            "-p",
            "8080:8080", // Metrics port
            "-e",
            "DATABASE_MAX_CONN=25",
            "-e",
            "TOPIC=pragma-data", // Kafka
            "-e",
            "KAFKA_BROKERS=pragma-data",
            "-e",
            &format!(
                "OFFCHAIN_DATABASE_URL={}",
                db_connection_url("test-offchain-db", offchain_port)
            ),
            "-e",
            &format!(
                "ONCHAIN_DATABASE_URL={}",
                db_connection_url("test-onchain-db", onchain_port)
            ),
            "-e",
            "METRICS_PORT=8080",
            "pragma-node-e2e",
        ])
        .output()
        .await
        .expect("Failed to run Docker container");

    if !output.status.success() {
        tracing::error!("Unable to run pragma-node container:");
        tracing::error!("{}", String::from_utf8(output.stderr).unwrap());
        panic!("Nope :)");
    }

    wait_for_pragma_node_to_be_ready().await;
}

async fn wait_for_pragma_node_to_be_ready() {
    let max_retries = 20;
    let retry_interval = Duration::from_secs(15);
    let node_url = "localhost:3000";

    sleep(Duration::from_secs(10)).await;

    for attempt in 1..=max_retries {
        match TcpStream::connect(node_url).await {
            Ok(_) => {
                // Delay to ensure migrations are applied
                sleep(Duration::from_secs(10)).await;
                return;
            }
            _ => {
                if attempt == max_retries {
                    panic!("pragma-node failed to start after {} attempts", max_retries);
                }
                tracing::debug!(
                    "Waiting for pragma-node to be ready (attempt {}/{})",
                    attempt,
                    max_retries
                );
                sleep(retry_interval).await;
            }
        }
    }
}

fn db_connection_url(host: &str, db_port: u16) -> String {
    format!(
        "postgres://postgres:test-password@{}:{}/pragma",
        host, db_port
    )
}

fn pragma_node_dockerfile_path() -> PathBuf {
    let mut current_dir = current_dir().unwrap();
    if current_dir.ends_with("tests") {
        current_dir = current_dir.join("..");
    }
    current_dir
        .join("infra")
        .join("pragma-node")
        .join("Dockerfile")
}
