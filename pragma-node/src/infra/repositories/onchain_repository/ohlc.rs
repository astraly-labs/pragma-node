use deadpool_diesel::postgres::Pool;
use diesel::RunQueryDsl;

use pragma_common::types::{DataType, Interval, Network};
use pragma_entities::error::InfraError;

use crate::infra::repositories::entry_repository::{OHLCEntry, OHLCEntryRaw};

use super::get_onchain_ohlc_table_name;

// Only works for Spot for now - since we only store spot entries on chain.
pub async fn get_ohlc(
    pool: &Pool,
    network: Network,
    pair_id: String,
    interval: Interval,
    data_to_retrieve: u64,
) -> Result<Vec<OHLCEntry>, InfraError> {
    let raw_sql = format!(
        r"
        SELECT
            ohlc_bucket AS time,
            open,
            high,
            low,
            close
        FROM
            {table_name}
        WHERE
            pair_id = $1
        ORDER BY
            time DESC
        LIMIT {data_to_retrieve};
        ",
        table_name = get_onchain_ohlc_table_name(network, DataType::SpotEntry, interval)?,
    );

    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .load::<OHLCEntryRaw>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    let entries: Vec<OHLCEntry> = raw_entries
        .into_iter()
        .map(|raw_entry| OHLCEntry {
            time: raw_entry.time,
            open: raw_entry.open,
            high: raw_entry.high,
            low: raw_entry.low,
            close: raw_entry.close,
        })
        .collect();

    Ok(entries)
}
