use rdkafka::config::ClientConfig;
use rdkafka::producer::future_producer::OwnedDeliveryResult;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::sync::LazyLock;

pub static KAFKA_PRODUCER: LazyLock<FutureProducer> = LazyLock::new(|| {
    let brokers = std::env::var("KAFKA_BROKERS").expect("can't load kafka brokers");
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");
    producer
});

pub async fn send_message(topic: &str, message: &[u8], key: &str) -> OwnedDeliveryResult {
    let delivery_status = KAFKA_PRODUCER.send(
        FutureRecord::to(topic).payload(message).key(key),
        std::time::Duration::from_secs(0),
    );
    delivery_status.await
}
