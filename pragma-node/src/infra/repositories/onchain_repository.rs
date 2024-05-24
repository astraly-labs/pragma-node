use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_monitoring::models::SpotEntry;

use crate::handlers::entries::OnchainEntry;

use diesel::RunQueryDsl;

pub async fn get_components_for_pair(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    mut timestamp: u64,
) -> Result<Vec<OnchainEntry>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    // substract one hour from the timestamp
    // TODO(akhercha): do this directly in the SQL request?
    timestamp -= 3600;

    let raw_sql = r#"
        SELECT DISTINCT * 
        FROM spot_entry 
        WHERE pair_id = $1 
        -- Only consider entries from the last hour
        AND timestamp >= to_timestamp($2)
        ORDER BY timestamp DESC;
    "#;

    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::BigInt, _>(timestamp as i64)
                .load::<SpotEntry>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    // Raise an error if no entries are found - shouldn't happen
    raw_entries.first().ok_or(InfraError::NotFound)?;

    // Adapt SpotEntry to OnchainEntry
    let entries: Vec<OnchainEntry> = raw_entries
        .iter()
        .map(|raw_entry: &SpotEntry| OnchainEntry {
            publisher: raw_entry.publisher.clone(),
            source: raw_entry.source.clone(),
            price: raw_entry.price.to_string(),
            tx_hash: raw_entry.transaction_hash.clone(),
            timestamp: raw_entry.timestamp.and_utc().timestamp() as u64,
        })
        .collect();

    Ok(entries)
}
