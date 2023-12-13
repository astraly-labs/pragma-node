use lazy_static::lazy_static;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::producer::future_producer::OwnedDeliveryResult;


lazy_static! {
    static ref KAFKA_PRODUCER: FutureProducer = {
        ClientConfig::new()
            .set("bootstrap.servers", "localhost:29092")
            .create()
            .expect("can't create kafka producer")
    };
}

pub async fn send_message(topic: &str, message: &[u8]) -> OwnedDeliveryResult {
    let delivery_status = KAFKA_PRODUCER.send(
        FutureRecord::to(topic)
            .payload(message)
            .key("first-data"),
        std::time::Duration::from_secs(0),
    );
    delivery_status.await
}