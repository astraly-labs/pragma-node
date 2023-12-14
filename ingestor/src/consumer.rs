use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{BaseConsumer, CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::message::{Headers, Message};
use tracing::{info, warn};
use tokio::sync::mpsc::UnboundedSender;
use crate::config::CONFIG;
pub async fn consume(tx: UnboundedSender<Vec<u8>>) {

    let consumer: BaseConsumer = ClientConfig::new()
        .set("group.id", &CONFIG.kafka.group_id)
        .set("bootstrap.servers", &CONFIG.kafka.brokers.join(","))
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create()
        .expect("Consumer creation failed");

    consumer
        .subscribe(&[&CONFIG.kafka.topic])
        .expect("Can't subscribe to specified topics");

    info!("start consuming...");

    for message in consumer.iter() {
        match message {
            Err(e) => warn!("Kafka error: {}", e),
            Ok(m) => {
                if let Some(payload) = m.payload() {
                    tx.send(payload.to_vec()).unwrap();
                }
                // auto commit ?
                consumer.commit_message(&m, CommitMode::Async).unwrap();
            }
        }
    }
}