use pragma_entities::EntryError;
use serde::Serialize;

use crate::infra::kafka;

/// Publish entries to Kafka
///
/// Arguments:
/// * `entries`: Vector of entries to publish (must implement Serialize)
/// * `topic`: Kafka topic
/// * `publisher_name`: Publisher name
///
/// Returns:
/// * `()`: Nothing
/// * `EntryError::PublishData`: Error if something goes wrong
pub async fn publish_to_kafka<T>(
    entries: Vec<T>,
    topic: String,
    publisher_name: &str,
) -> Result<(), EntryError>
where
    T: Serialize,
{
    let data = serde_json::to_vec(&entries).map_err(|e| EntryError::PublishData(e.to_string()))?;

    if let Err(e) = kafka::send_message(&topic, &data, publisher_name).await {
        tracing::error!("Error sending message to kafka: {:?}", e);
        return Err(EntryError::PublishData(String::from(
            "Error sending message to kafka",
        )));
    };

    Ok(())
}
