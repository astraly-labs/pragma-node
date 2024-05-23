use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_monitoring::models::SpotEntry;

use diesel::RunQueryDsl;

// TODO(akhercha): Created for testing purposes - remove when not needed
pub async fn _get_latest_spot(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
) -> Result<SpotEntry, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_sql = r#"
        -- get the latest spot entry for a given pair
        SELECT *
        FROM spot_entry
        WHERE pair_id = $1
        ORDER BY timestamp DESC
        LIMIT 1
    "#;

    let raw_entry = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .load::<SpotEntry>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let raw_entry: &SpotEntry = raw_entry.first().ok_or(InfraError::NotFound)?;

    let entry: SpotEntry = SpotEntry {
        network: raw_entry.network.clone(),
        pair_id: raw_entry.pair_id.clone(),
        data_id: raw_entry.data_id.clone(),
        block_hash: raw_entry.block_hash.clone(),
        block_number: raw_entry.block_number,
        block_timestamp: raw_entry.block_timestamp,
        transaction_hash: raw_entry.transaction_hash.clone(),
        price: raw_entry.price.clone(),
        timestamp: raw_entry.timestamp,
        publisher: raw_entry.publisher.clone(),
        source: raw_entry.source.clone(),
        volume: raw_entry.volume.clone(),
        _cursor: raw_entry._cursor,
    };
    Ok(entry)
}
