use lazy_static::lazy_static;
use rdkafka::config::ClientConfig;
use rdkafka::producer::future_producer::OwnedDeliveryResult;
use rdkafka::producer::{FutureProducer, FutureRecord};

lazy_static! {
    static ref KAFKA_PRODUCER: FutureProducer = {
        ClientConfig::new()
            // soon loaded by config
            .set("bootstrap.servers", "localhost:29092")
            .create()
            .expect("can't create kafka producer")
    };
}

pub async fn send_message(topic: &str, message: &[u8], key: &str) -> OwnedDeliveryResult {
    let delivery_status = KAFKA_PRODUCER.send(
        FutureRecord::to(topic).payload(message).key(key),
        std::time::Duration::from_secs(0),
    );
    delivery_status.await
}
