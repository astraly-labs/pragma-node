use std::path::{Path, PathBuf};

use tokio::process::Command;

const DOCKER_BINARY: &str = "docker";

#[derive(Debug, Clone, Default)]
pub struct ImageBuilder {
    build_name: String,
    dockerfile: PathBuf,
}

impl ImageBuilder {
    pub fn with_build_name(mut self, name: &str) -> Self {
        self.build_name = name.to_owned();
        self
    }

    pub fn with_dockerfile(mut self, dockerfile_path: &Path) -> Self {
        self.dockerfile = dockerfile_path.to_path_buf();
        self
    }

    pub async fn build(&self) {
        let output = Command::new(DOCKER_BINARY)
            .args([
                "buildx",
                "build",
                "--file",
                self.dockerfile.to_str().unwrap(),
                "--force-rm",
                "--tag",
                &self.build_name,
                "..",
            ])
            .output()
            .await
            .expect("Failed to execute Docker build command");

        if !output.status.success() {
            tracing::error!("{}", String::from_utf8(output.stderr).unwrap());
            panic!("Failed to build image for {}", &self.build_name);
        }
    }
}
