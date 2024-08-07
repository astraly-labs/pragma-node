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
