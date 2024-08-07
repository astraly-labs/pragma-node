use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::zookeeper::Zookeeper;

#[rstest::fixture]
pub async fn setup_zookeeper() -> ContainerAsync<Zookeeper> {
    Zookeeper::default()
        .with_name("confluentinc/cp-zookeeper")
        .with_tag("latest")
        .with_env_var("ZOOKEEPER_CLIENT_PORT", "2181")
        .with_env_var("ZOOKEEPER_TICK_TIME", "2000")
        .with_mapped_port(2181, 2181_u16.tcp())
        .with_network("pragma-tests-zookeeper-network")
        .with_container_name("test-zookeeper")
        .start()
        .await
        .unwrap()
}
