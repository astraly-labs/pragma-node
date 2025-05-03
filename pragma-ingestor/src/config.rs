use clap::Parser;
use std::sync::LazyLock;

pub(crate) static CONFIG: LazyLock<Ingestor> = LazyLock::new(load_configuration);

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Ingestor {
    /// Kafka broker ID
    #[arg(
        long,
        env = "KAFKA_BROKER_ID",
        default_value = "kafka.devnet.pragma.build:9092"
    )]
    pub(crate) kafka_broker_id: String,

    /// Kafka consumer group ID
    #[arg(long, env = "KAFKA_GROUP_ID", default_value = "pragma-ingestor")]
    pub(crate) kafka_group_id: String,

    /// Number of consumers to run
    #[arg(long, env = "NUM_CONSUMERS", default_value = "1")]
    pub(crate) num_consumers: usize,

    /// Channel capacity for message queues
    #[arg(long, env = "CHANNEL_CAPACITY", default_value = "1000000")]
    pub(crate) channel_capacity: usize,

    /// Publisher name for entries
    #[arg(long, env = "PUBLISHER_NAME", default_value = "PRAGMA")]
    pub(crate) publisher_name: String,

    /// OpenTelemetry endpoint for telemetry data
    #[arg(long, env = "OTEL_EXPORTER_OTLP_ENDPOINT")]
    pub(crate) otel_endpoint: Option<String>,
}

pub(crate) fn load_configuration() -> Ingestor {
    Ingestor::parse()
}
