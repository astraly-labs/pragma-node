use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::common::containers::pragma_node::{METRICS_PORT, SERVER_PORT};

/// Maximum time to wait for the service to be ready
const MAX_WAIT_DURATION: Duration = Duration::from_secs(300);

pub async fn setup_pragma_node_with_cargo(
    offchain_db_port: u16,
    onchain_db_port: u16,
) -> tokio::process::Child {
    // Set required environment variables for local mode
    let env_vars = [
        ("DATABASE_MAX_CONN", "25"),
        ("TOPIC", "pragma-data"),
        ("KAFKA_BROKERS", "pragma-data"),
        ("PORT", &SERVER_PORT.to_string()),
        ("METRICS_PORT", &METRICS_PORT.to_string()),
        (
            "OFFCHAIN_DATABASE_URL",
            &format!(
                "postgres://postgres:test-password@localhost:{}/pragma",
                offchain_db_port
            ),
        ),
        (
            "ONCHAIN_DATABASE_URL",
            &format!(
                "postgres://postgres:test-password@localhost:{}/pragma",
                onchain_db_port
            ),
        ),
        ("MODE", "dev"),
    ];

    // Go up one level to the workspace root if we're located in the `tests` crate
    let mut workspace_root = std::env::current_dir().expect("Failed to get current directory");
    if let Ok(dir) = std::env::current_dir() {
        if dir.ends_with("tests") {
            workspace_root.pop();
        }
    }

    // Start the local pragma-node process with output piping
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.args(["run", "--release", "--bin", "pragma-node"])
        .current_dir(workspace_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Add all the required ENV variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let mut child = cmd.spawn().expect("Failed to start pragma-node");

    // Set up output readers
    let stdout = BufReader::new(child.stdout.take().expect("Failed to capture stdout"));
    let stderr = BufReader::new(child.stderr.take().expect("Failed to capture stderr"));

    let mut stdout_lines = stdout.lines();
    let mut stderr_lines = stderr.lines();

    // Look for signals that indicate the service is ready
    loop {
        tokio::select! {
            line = stdout_lines.next_line() => {
                if let Ok(Some(line)) = line {
                    println!("stdout: {}", line);
                    if line.contains("🚀 API started at") {
                        println!("Pragma node is ready!");
                        break;
                    }
                }
            }
            line = stderr_lines.next_line() => {
                if let Ok(Some(line)) = line {
                    println!("stderr: {}", line);
                    if line.contains("Finished release") {
                        println!("Build completed, waiting for server startup...");
                    }
                }
            }
            // Timeout after MAX_WAIT_DURATION
            _ = tokio::time::sleep(MAX_WAIT_DURATION) => {
                child.kill().await.expect("Failed to kill process on timeout");
                panic!("Timeout waiting for pragma-node to start");
            }
        }
    }

    child
}
