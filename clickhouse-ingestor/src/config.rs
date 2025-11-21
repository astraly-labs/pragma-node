use clap::Parser;
use std::sync::LazyLock;

pub(crate) static CONFIG: LazyLock<Config> = LazyLock::new(load_configuration);

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Config {
    /// Kafka broker ID
    #[arg(
        long,
        env = "KAFKA_BROKER_ID",
        default_value = "kafka.devnet.pragma.build:9092"
    )]
    pub kafka_broker_id: String,

    /// Kafka consumer group ID
    #[arg(long, env = "KAFKA_GROUP_ID", default_value = "clickhouse-ingestor")]
    pub kafka_group_id: String,

    /// Channel capacity for message queues
    #[arg(long, env = "CHANNEL_CAPACITY", default_value = "100000")]
    pub channel_capacity: usize,
    
    /// ClickHouse URL
    #[arg(long, env = "CLICKHOUSE_URL", default_value = "http://localhost:8123")]
    pub clickhouse_url: String,

    /// ClickHouse database name
    #[arg(long, env = "CLICKHOUSE_DATABASE", default_value = "default")]
    pub clickhouse_database: String,

    /// Pairs to ingest (comma-separated, e.g., "BTC/USD,ETH/USD")
    #[arg(
        long,
        env = "PAIRS",
        value_delimiter = ',',
        default_values_t = default_pairs()
    )]
    pub pairs: Vec<String>,

    /// OpenTelemetry endpoint for telemetry data
    #[arg(long, env = "OTEL_EXPORTER_OTLP_ENDPOINT")]
    pub otel_endpoint: Option<String>,

    /// Batch flush interval in milliseconds
    #[arg(long, env = "FLUSH_INTERVAL_MS", default_value = "500")]
    pub flush_interval_ms: u64,
}

fn default_pairs() -> Vec<String> {
    vec![
        "TSLA/USD".to_string(),
        "EUR/USD".to_string(),
        "XAU/USD".to_string(),
        "SPX500M/USD".to_string(),
        "XBR/USD".to_string(),
        "TECH100M/USD".to_string(),
        "USD/JPY".to_string(),
        "XAG/USD".to_string(),
        "XPL/USD".to_string(),
    ]
}

pub(crate) fn load_configuration() -> Config {
    Config::parse()
}
