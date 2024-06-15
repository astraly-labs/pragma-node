use crate::config::CONFIG;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info};

pub async fn consume(tx: UnboundedSender<Vec<u8>>) {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", &CONFIG.group_id)
        .set("bootstrap.servers", CONFIG.brokers.join(","))
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create()
        .expect("Consumer creation failed");

    consumer
        .subscribe(&[&CONFIG.topic])
        .expect("Can't subscribe to specified topics");

    info!(
        "start consuming at {}({})",
        CONFIG.brokers.join(","),
        &CONFIG.topic
    );

    loop {
        if let Ok(ref message) = consumer.recv().await {
            if let Some(payload) = message.payload() {
                if let Err(e) = tx.send(payload.to_vec()) {
                    error!("cannot send message to bootstrap handler : {}.", e);
                }
            }

            if let Err(e) = consumer.commit_message(message, CommitMode::Async) {
                error!("cannot commit message : {:?}", e);
            }
        }
    }
}
