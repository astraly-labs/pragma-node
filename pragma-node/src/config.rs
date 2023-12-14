use std::env;

use dotenvy::dotenv;
use tokio::sync::OnceCell;

#[derive(Debug)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug)]
struct DatabaseConfig {
    url: String,
}

#[derive(Debug)]
struct KafkaConfig {
    brokers: Vec<String>,
    topic: String,
    group_id: String,
}

#[derive(Debug)]
pub struct Config {
    server: ServerConfig,
    db: DatabaseConfig,
    kafka: KafkaConfig,
}

impl Config {
    pub fn db_url(&self) -> &str {
        &self.db.url
    }

    pub fn server_host(&self) -> &str {
        &self.server.host
    }

    pub fn server_port(&self) -> u16 {
        self.server.port
    }

    pub fn kafka_topic(&self) -> &str {
        &self.kafka.topic
    }
}

pub static CONFIG: OnceCell<Config> = OnceCell::const_new();

async fn init_config() -> Config {
    dotenv().ok();
    let server_config = ServerConfig {
        host: env::var("HOST").unwrap_or_else(|_| String::from("127.0.0.1")),
        port: env::var("PORT")
            .unwrap_or_else(|_| String::from("3000"))
            .parse::<u16>()
            .unwrap(),
    };

    let database_config = DatabaseConfig {
        url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
    };

    let kafka_config = KafkaConfig {
        brokers: vec![
            env::var("KAFKA_BROKERS").unwrap_or_else(|_| String::from("pragma-kafka:9092"))
        ],
        topic: env::var("KAFKA_TOPIC").unwrap_or_else(|_| String::from("pragma-data")),
        group_id: env::var("KAFKA_GROUP_ID").unwrap_or_else(|_| String::from("pragma-data")),
    };

    Config {
        server: server_config,
        db: database_config,
        kafka: kafka_config,
    }
}

pub async fn config() -> &'static Config {
    CONFIG.get_or_init(init_config).await
}
