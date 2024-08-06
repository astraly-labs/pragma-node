use bigdecimal::BigDecimal;
use deadpool_diesel::postgres::Pool;
use diesel::sql_types::{Numeric, Timestamp, VarChar};
use diesel::{Queryable, QueryableByName, RunQueryDsl};

use pragma_common::types::Network;
use pragma_entities::error::{adapt_infra_error, InfraError};

use crate::handlers::onchain::get_checkpoints::Checkpoint;
use crate::utils::format_bigdecimal_price;


#[derive(Queryable, QueryableByName)]
struct RawCheckpoint {
    #[diesel(sql_type = VarChar)]
    pub transaction_hash: String,
    #[diesel(sql_type = Numeric)]
    pub price: BigDecimal,
    #[diesel(sql_type = Timestamp)]
    pub timestamp: chrono::NaiveDateTime,
    #[diesel(sql_type = VarChar)]
    pub sender_address: String,
}

impl RawCheckpoint {
    pub fn to_checkpoint(&self, decimals: u32) -> Checkpoint {
        Checkpoint {
            tx_hash: self.transaction_hash.clone(),
            price: format_bigdecimal_price(self.price.clone(), decimals),
            timestamp: self.timestamp.and_utc().timestamp() as u64,
            sender_address: self.sender_address.clone(),
        }
    }
}

pub async fn get_checkpoints(
    pool: &Pool,
    network: Network,
    pair_id: String,
    decimals: u32,
    limit: u64,
) -> Result<Vec<Checkpoint>, InfraError> {
    let table_name = match network {
        Network::Mainnet => "mainnet_spot_checkpoints",
        Network::Sepolia => "spot_checkpoints",
    };
    let raw_sql = format!(
        r#"
        SELECT
            transaction_hash,
            price,
            timestamp,
            sender_address
        FROM
            {table_name}
        WHERE
            pair_id = $1
        ORDER BY timestamp DESC
        LIMIT $2;
    "#,
        table_name = table_name
    );

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_checkpoints = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::BigInt, _>(limit as i64)
                .load::<RawCheckpoint>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let checkpoints: Vec<Checkpoint> = raw_checkpoints
        .into_iter()
        .map(|raw_checkpoint| raw_checkpoint.to_checkpoint(decimals))
        .collect();
    Ok(checkpoints)
}
