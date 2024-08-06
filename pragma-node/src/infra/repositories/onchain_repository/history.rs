use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use deadpool_diesel::postgres::Pool;
use diesel::{prelude::QueryableByName, RunQueryDsl};

use pragma_common::types::{DataType, Interval, Network};
use pragma_entities::error::{adapt_infra_error, InfraError};
use serde::Serialize;

use crate::types::timestamp::TimestampRange;

use super::get_onchain_aggregate_table_name;

#[derive(Serialize, QueryableByName, Clone, Debug)]
pub struct HistoricalEntryRaw {
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub pair_id: String,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub timestamp: NaiveDateTime,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    pub median_price: BigDecimal,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub nb_sources_aggregated: i64,
}

pub async fn get_historical_aggregated_entries(
    pool: &Pool,
    network: Network,
    pair_id: String,
    timestamp: TimestampRange,
    chunk_interval: Interval,
) -> Result<Vec<HistoricalEntryRaw>, InfraError> {
    let (start_timestamp, end_timestamp) = {
        let range = timestamp.0;
        (*range.start(), *range.end())
    };

    let raw_sql = format!(
        r#"
        SELECT
            pair_id,
            bucket AS timestamp,
            median_price,
            num_sources AS nb_sources_aggregated
        FROM
            {table_name}
        WHERE
            pair_id = $1
            AND bucket >= to_timestamp($2)
            AND bucket <= to_timestamp($3)
        ORDER BY
            bucket ASC
        "#,
        table_name =
            get_onchain_aggregate_table_name(network, DataType::SpotEntry, chunk_interval)?,
    );

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(&pair_id)
                .bind::<diesel::sql_types::BigInt, _>(start_timestamp)
                .bind::<diesel::sql_types::BigInt, _>(end_timestamp)
                .load::<HistoricalEntryRaw>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(raw_entries)
}
