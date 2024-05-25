use axum::extract::{Query, State};
use axum::Json;

use pragma_entities::EntryError;

use crate::handlers::entries::{GetOnchainParams, GetOnchainResponse};
use crate::infra::repositories::onchain_repository::{
    compute_price, get_last_updated_timestamp, get_sources_for_pair,
};
use crate::utils::PathExtractor;
use crate::AppState;

use super::utils::currency_pair_to_pair_id;

#[utoipa::path(
    get,
    path = "/node/v1/onchain/{base}/{quote}",
    responses(
        (status = 200, description = "Get the onchain price", body = GetOnchainResponse)
    ),
    params(
        ("base" = String, Path, description = "Base Asset"),
        ("quote" = String, Path, description = "Quote Asset"),
        ("network" = Network, Query, description = "Network"),
        ("aggregation" = AggregationMode, Query, description = "Aggregation Mode"),
        ("timestamp" = Option<u64>, Query, description = "Timestamp")
    ),
)]
pub async fn get_onchain(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetOnchainParams>,
) -> Result<Json<GetOnchainResponse>, EntryError> {
    tracing::info!("Received get onchain entry request for pair {:?}", pair);

    let pair_id: String = currency_pair_to_pair_id(&pair.0, &pair.1);
    let aggregation_mode = params.aggregation;
    let network = params.network;

    let now = chrono::Utc::now().naive_utc().and_utc().timestamp() as u64;
    let timestamp = if let Some(timestamp) = params.timestamp {
        timestamp
    } else {
        now
    };

    let sources = get_sources_for_pair(&state.postgres_pool, network, pair_id.clone(), timestamp)
        .await
        .map_err(|_| EntryError::InternalServerError)?;

    // TODO(akhercha): ⚠ gives different result than onchain oracle
    let last_updated_timestamp = get_last_updated_timestamp(&state.postgres_pool, pair_id.clone())
        .await
        .map_err(|_| EntryError::InternalServerError)?;

    // TODO(akhercha): compute directly the aggregation within the SQL query
    // TODO(akhercha): ⚠ returns a slightly different price than onchain oracle
    let price = if sources.is_empty() {
        "0".to_string()
    } else {
        compute_price(&sources, aggregation_mode).unwrap()
    };

    let res = GetOnchainResponse {
        pair_id,
        last_updated_timestamp,
        price,
        decimals: 8,
        nb_sources_aggregated: sources.len() as u32,
        // Only asset type used for now is Crypto
        asset_type: "Crypto".to_string(),
        components: sources,
    };
    Ok(Json(res))
}
