use testcontainers::core::ExecCommand;
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
        .with_network("pragma-tests-network")
        .with_container_name(KAFKA_CONTAINER_NAME)
        .start()
        .await
        .unwrap()
}

pub async fn init_kafka_topics(kafka: &ContainerAsync<Kafka>) {
    let cmd_create_pragma_data = ExecCommand::new(vec![
        "kafka-topics",
        "--bootstrap-server",
        "localhost:9092",
        "--topic",
        "pragma-data",
        "--create",
        "--partitions",
        "1",
        "--replication-factor",
        "1",
    ]);
    kafka.exec(cmd_create_pragma_data).await.unwrap();

    let cmd_create_consumer_offsets = ExecCommand::new(vec![
        "kafka-topics",
        "--bootstrap-server",
        "localhost:9092",
        "--topic",
        "__consumer_offsets",
        "--create",
        "--partitions",
        "1",
        "--replication-factor",
        "1",
    ]);
    kafka.exec(cmd_create_consumer_offsets).await.unwrap();
}
