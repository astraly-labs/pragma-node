use tokio::process::Command;

pub async fn kill_and_remove_container(container_name: &str) {
    // Kill the container
    let kill_output = Command::new("docker")
        .args(["kill", container_name])
        .output()
        .await
        .expect("Failed to kill Docker container");

    if kill_output.status.success() {
        panic!(
            "Failed to kill pragma-node container: {}",
            String::from_utf8_lossy(&kill_output.stderr)
        );
    }

    // Remove the container
    let rm_output = Command::new("docker")
        .args(["rm", container_name])
        .output()
        .await
        .expect("Failed to remove Docker container");

    if !rm_output.status.success() {
        panic!(
            "Failed to remove pragma-node container: {}",
            String::from_utf8_lossy(&rm_output.stderr)
        );
    }
}
