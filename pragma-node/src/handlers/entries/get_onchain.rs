use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use pragma_entities::EntryError;

use crate::handlers::entries::{AggregationMode, GetOnchainResponse};
use crate::infra::repositories::onchain_repository::{
    compute_price, get_last_updated_timestamp, get_sources_for_pair,
};
use crate::utils::PathExtractor;
use crate::AppState;

use super::utils::currency_pair_to_pair_id;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct GetOnchainParams {
    pub aggregation: Option<AggregationMode>,
    pub timestamp: Option<u64>,
}

impl Default for GetOnchainParams {
    fn default() -> Self {
        Self {
            aggregation: Some(AggregationMode::default()),
            timestamp: Some(chrono::Utc::now().naive_utc().and_utc().timestamp_millis() as u64),
        }
    }
}

#[utoipa::path(
    get,
    path = "/node/v1/onchain/{base}/{quote}",
    responses(
        (status = 200, description = "Get the latest onchain entry", body = [GetOnchainResponse])
    ),
    params(
        ("base" = String, Path, description = "Base Asset"),
        ("quote" = String, Path, description = "Quote Asset"),
        GetOnchainParams,
    ),
)]
pub async fn get_onchain(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetOnchainParams>,
) -> Result<Json<GetOnchainResponse>, EntryError> {
    tracing::debug!("Params: {:?}", params);
    tracing::info!("Received get onchain entry request for pair {:?}", pair);
    let pair_id: String = currency_pair_to_pair_id(&pair.0, &pair.1);

    // TODO(akhercha): Arguments passed in Swagger UI are not being passed to the handler
    // let now = chrono::Utc::now().naive_utc().and_utc().timestamp_millis() as u64;
    // let timestamp = if let Some(timestamp) = params.timestamp {
    //     timestamp
    // } else {
    //     now
    // };
    // TODO(akhercha): Debugging purposes only - set to last timestamp in the test DB
    let timestamp: u64 = 1706078546;

    let agg_mode = if let Some(aggregation_mode) = params.aggregation {
        aggregation_mode
    } else {
        AggregationMode::Median
    };

    let sources = get_sources_for_pair(&state.postgres_pool, pair_id.clone(), timestamp)
        .await
        .map_err(|_| EntryError::InternalServerError)?;

    // TODO(akhercha): wrong - gives different result than onchain oracle
    let last_updated_timestamp = get_last_updated_timestamp(&state.postgres_pool, pair_id.clone())
        .await
        .map_err(|_| EntryError::InternalServerError)?;

    // TODO(akhercha): returns a slightly different price than onchain oracle
    let price = compute_price(&sources, agg_mode);

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
