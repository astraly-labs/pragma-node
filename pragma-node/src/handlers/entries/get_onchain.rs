use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use pragma_entities::EntryError;

use crate::handlers::entries::{AggregationMode, GetOnchainResponse};
use crate::infra::repositories::onchain_repository::get_components_for_pair;
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
    let timestamp: u64 = 1706078361;

    let _agg_mode = if let Some(aggregation_mode) = params.aggregation {
        aggregation_mode
    } else {
        AggregationMode::Median
    };

    let pair_components = get_components_for_pair(&state.postgres_pool, pair_id.clone(), timestamp)
        .await
        .map_err(|_| EntryError::InternalServerError)?;

    let res: GetOnchainResponse = GetOnchainResponse {
        pair_id,
        // TODO(akhercha): compute those parameters in onchain_repository - maybe they already exist
        last_updated_timestamp: 0,
        price: "0".to_string(),
        decimals: 8,
        nb_sources_aggregated: pair_components.len() as u32,
        // Only asset type used for now is Crypto
        asset_type: "Crypto".to_string(),
        components: pair_components,
    };
    Ok(Json(res))
}
