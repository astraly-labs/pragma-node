use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::kafka::Kafka;

#[rstest::fixture]
pub async fn setup_kafka() -> ContainerAsync<Kafka> {
    Kafka::default()
        .with_name("confluentinc/cp-kafka")
        .with_tag("latest")
        .with_env_var("KAFKA_BROKER_ID", "1")
        .with_env_var("KAFKA_ZOOKEEPER_CONNECT", "test-zookeeper:2181")
        .with_env_var(
            "KAFKA_ADVERTISED_LISTENERS",
            "PLAINTEXT://test-kafka:9092,PLAINTEXT_E://test-kafka:29092",
        )
        .with_env_var(
            "KAFKA_LISTENER_SECURITY_PROTOCOL_MAP",
            "PLAINTEXT:PLAINTEXT,PLAINTEXT_HOST:PLAINTEXT,PLAINTEXT_E:PLAINTEXT",
        )
        .with_env_var("KAFKA_INTER_BROKER_LISTENER_NAME", "PLAINTEXT")
        .with_env_var("KAFKA_AUTO_CREATE_TOPICS_ENABLE", "true")
        .with_env_var("KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR", "3")
        .with_mapped_port(29092, 29092_u16.tcp())
        .with_mapped_port(9092, 9092_u16.tcp())
        .with_network("pragma-tests-kafka-network")
        .with_network("pragma-tests-zookeeper-network")
        .with_container_name("test-kafka")
        .start()
        .await
        .unwrap()
}
