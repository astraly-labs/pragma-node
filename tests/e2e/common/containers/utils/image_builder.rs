use std::path::PathBuf;

use bollard::image::BuildImageOptions;
use futures_util::StreamExt;
use tar::Builder;
use testcontainers::core::client::docker_client_instance;

#[derive(Debug, Clone, Default)]
pub struct ImageBuilder {
    build_name: String,
    dockerfile_dir: PathBuf,
}

impl ImageBuilder {
    pub fn with_build_name(mut self, name: &str) -> Self {
        self.build_name = name.to_owned();
        self
    }

    pub fn with_dockerfile_dir(mut self, dockerfile_dir: PathBuf) -> Self {
        self.dockerfile_dir = dockerfile_dir;
        self
    }

    pub async fn build(&self) {
        let docker = docker_client_instance().await.unwrap();

        // Create a tarball of the build context
        let tarball = self.create_tarball().unwrap();

        let options = BuildImageOptions::<String> {
            dockerfile: "Dockerfile".to_string(),
            t: self.build_name.clone(),
            rm: true,
            ..Default::default()
        };

        let mut build_stream = docker.build_image(options, None, Some(tarball.into()));
        build_stream.next().await.unwrap().unwrap();
    }

    fn create_tarball(&self) -> std::io::Result<Vec<u8>> {
        let mut tar_builder = Builder::new(Vec::new());
        tar_builder.append_dir_all(".", &self.dockerfile_dir)?;
        tar_builder.into_inner()
    }
}
