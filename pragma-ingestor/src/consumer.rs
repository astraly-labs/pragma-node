use crate::config::CONFIG;
use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{BaseConsumer, CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::message::{Headers, Message};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

pub async fn consume(tx: UnboundedSender<Vec<u8>>) {
    let consumer: BaseConsumer = ClientConfig::new()
        .set("group.id", &CONFIG.group_id)
        .set("bootstrap.servers", &CONFIG.brokers.join(","))
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "true")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create()
        .expect("Consumer creation failed");

    consumer
        .subscribe(&[&CONFIG.topic])
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
