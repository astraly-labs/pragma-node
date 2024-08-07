use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::kafka::Kafka;

pub const KAFKA_CONTAINER_NAME: &str = "test-kafka";

#[rstest::fixture]
pub async fn setup_kafka() -> ContainerAsync<Kafka> {
    Kafka::default()
        .with_name("confluentinc/cp-kafka")
        .with_tag("latest")
        .with_env_var("KAFKA_AUTO_CREATE_TOPICS_ENABLE", "true")
        .with_mapped_port(29092, 29092_u16.tcp())
        .with_mapped_port(9093, 9093_u16.tcp())
        .with_network("pragma-tests-zookeeper-network")
        .with_network("pragma-tests-kafka-network")
        .with_container_name(KAFKA_CONTAINER_NAME)
        .start()
        .await
        .unwrap()
}
