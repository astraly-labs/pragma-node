use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use deadpool_diesel::postgres::Pool;
use diesel::{
    QueryableByName, RunQueryDsl,
    sql_types::{BigInt, Nullable, Numeric, Text, Timestamptz},
};
use pragma_common::{InstrumentType, Pair, starknet::StarknetNetwork};
use pragma_entities::{InfraError, models::entries::timestamp::TimestampRange};

use super::get_onchain_table_name;

#[derive(QueryableByName)]
pub struct RawObservation {
    #[diesel(sql_type = Text)]
    pub publisher: String,
    #[diesel(sql_type = Text)]
    pub source: String,
    #[diesel(sql_type = Timestamptz)]
    pub timestamp: NaiveDateTime,
    #[diesel(sql_type = Numeric)]
    pub price: BigDecimal,
    #[diesel(sql_type = Text)]
    pub tx_hash: String,
}

pub async fn get_observations(
    pool: &Pool,
    network: StarknetNetwork,
    pair: &Pair,
    range: TimestampRange,
    publisher: Option<String>,
    source: Option<String>,
    bucket_seconds: i64,
) -> Result<Vec<RawObservation>, InfraError> {
    let table = get_onchain_table_name(network, InstrumentType::Spot);
    let selection = if bucket_seconds == 0 {
        format!(
            "SELECT publisher, source, timestamp, price, transaction_hash AS tx_hash FROM {table}"
        )
    } else {
        format!(
            "SELECT DISTINCT ON (publisher, source, FLOOR(EXTRACT(EPOCH FROM timestamp) / $6)) publisher, source, timestamp, price, transaction_hash AS tx_hash FROM {table}"
        )
    };
    let ordering = if bucket_seconds == 0 {
        ""
    } else {
        "ORDER BY publisher, source, FLOOR(EXTRACT(EPOCH FROM timestamp) / $6), timestamp DESC, transaction_hash DESC"
    };
    // $6 is bound in both branches, including raw mode, so the bind count is stable.
    let sql = format!(
        r"
        SELECT * FROM (
            {selection}
            WHERE pair_id = $1 AND timestamp >= to_timestamp($2) AND timestamp <= to_timestamp($3)
                AND ($4::text IS NULL OR publisher = $4)
                AND ($5::text IS NULL OR source = $5) AND $6::bigint >= 0
            {ordering}
        ) observations
        ORDER BY timestamp ASC, publisher, source, tx_hash
        LIMIT 20001
    "
    );
    let pair_id = pair.to_pair_id();
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    conn.interact(move |conn| {
        diesel::sql_query(sql)
            .bind::<Text, _>(pair_id)
            .bind::<BigInt, _>(*range.0.start())
            .bind::<BigInt, _>(*range.0.end())
            .bind::<Nullable<Text>, _>(publisher)
            .bind::<Nullable<Text>, _>(source)
            .bind::<BigInt, _>(bucket_seconds)
            .load::<RawObservation>(conn)
    })
    .await
    .map_err(InfraError::DbInteractionError)?
    .map_err(InfraError::DbResultError)
}
