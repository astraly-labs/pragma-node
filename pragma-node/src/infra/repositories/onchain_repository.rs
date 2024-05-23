use crate::handlers::entries::AggregationMode;

use crate::infra::repositories::entry_repository::MedianEntry;

use pragma_entities::error::InfraError;

#[allow(dead_code)]
pub async fn routing(
    _pool: &deadpool_diesel::postgres::Pool,
    _pair_id: String,
    _timestamp: u64,
    _agg_mode: AggregationMode,
) -> Result<(MedianEntry, u32), InfraError> {
    todo!("not implemented yet")
}
