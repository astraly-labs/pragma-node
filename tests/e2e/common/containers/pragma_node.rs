use std::{
    env::current_dir, net::TcpStream, path::PathBuf, process::Command, thread, time::Duration,
};

// It is not possible yet to start local Dockefiles using the testcontainers
// rust crate.
// To bypass this, we start it using the `docker` command ourselves.
pub fn setup_pragma_node(offchain_port: u16, onchain_port: u16) {
    let dockerfile_path = pragma_node_dockerfile_path();

    // Build the pragma-node Docker image
    // TODO(akhercha): Assert that the docker command is installed?
    let output = Command::new("docker")
        .arg("build")
        .arg("--file")
        .arg(dockerfile_path)
        .arg("--force-rm")
        .arg("--tag")
        .arg("pragma-node-e2e")
        .arg(".")
        .output()
        .unwrap();

    if !output.status.success() {
        tracing::error!("Unable to build pragma-node-e2e");
        tracing::error!("{}", String::from_utf8(output.stderr).unwrap());
        panic!("Nope :)");
    } else {
        tracing::info!("Built pragma-node-e2e");
    }

    // Run the pragma-node Docker container with environment variables
    let output = Command::new("docker")
        .arg("run")
        .arg("-d") // Run in detached mode
        .arg("-p")
        .arg("3000:3000") // Node API port
        .arg("-p")
        .arg("8080:8080") // Metrics port
        .arg("-e")
        .arg("DATABASE_MAX_CONN=25")
        .arg("-e")
        .arg("TOPIC=pragma-data")
        .arg("-e")
        .arg("KAFKA_BROKERS=pragma-data")
        .arg("-e")
        .arg(format!(
            "OFFCHAIN_DATABASE_URL={}",
            db_connection_url(offchain_port)
        ))
        .arg("-e")
        .arg(format!(
            "ONCHAIN_DATABASE_URL={}",
            db_connection_url(onchain_port)
        ))
        .arg("-e")
        .arg("METRICS_PORT=8080")
        .arg("pragma-node-e2e")
        .output()
        .unwrap();

    if !output.status.success() {
        tracing::error!("Unable to run pragma-node container:");
        tracing::error!("{}", String::from_utf8(output.stderr).unwrap());
        panic!("Nope :)");
    } else {
        wait_for_pragma_node_to_be_ready();
        tracing::info!("Started pragma-node container");
    }
}

fn wait_for_pragma_node_to_be_ready() {
    tracing::info!("Waiting for pragma-node container to be ready...");
    let max_retries = 10;
    let retry_interval = Duration::from_secs(2);
    let port = 3000;

    for attempt in 1..=max_retries {
        match TcpStream::connect(("localhost", port)) {
            Ok(_) => {
                tracing::error!("pragma-node is now ready and listening on port {}", port);
                break;
            }
            Err(_) => {
                if attempt == max_retries {
                    panic!("pragma-node failed to start after {} attempts", max_retries);
                }
                tracing::error!(
                    "Waiting for pragma-node to be ready (attempt {}/{})",
                    attempt,
                    max_retries
                );
                thread::sleep(retry_interval);
            }
        }
    }
}

fn db_connection_url(db_port: u16) -> String {
    format!(
        "postgres://postgres:test-password@localhost:{}/pragma",
        db_port
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
