use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use pragma_entities::EntryError;

use crate::handlers::entries::{AggregationMode, GetOnchainEntryResponse};
use crate::infra::onchain::oracle::{get_data_median, GetDataMedianResponse};
use crate::infra::repositories::onchain_repository::get_components_for_pair;
use crate::utils::PathExtractor;
use crate::AppState;

use super::utils::currency_pair_to_pair_id;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct GetOnchainParams {
    pub aggregation: Option<AggregationMode>,
}

impl Default for GetOnchainParams {
    fn default() -> Self {
        Self {
            aggregation: Some(AggregationMode::default()),
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
) -> Result<Json<GetOnchainEntryResponse>, EntryError> {
    tracing::info!("Received get onchain entry request for pair {:?}", pair);
    let pair_id = currency_pair_to_pair_id(&pair.0, &pair.1);

    let now = chrono::Utc::now().naive_utc().and_utc().timestamp_millis() as u64;

    // TODO(akhercha): Currently only agg_mode used is Median
    let _agg_mode = if let Some(aggregation_mode) = params.aggregation {
        aggregation_mode
    } else {
        AggregationMode::Median
    };

    // TODO(akhercha): Call `get_data` with correct parameters
    let onchain_pair_median: GetDataMedianResponse =
        get_data_median(&state.network, pair_id.clone())
            .await
            .map_err(|_| EntryError::InternalServerError)?;

    let pair_components = get_components_for_pair(&state.postgres_pool, pair_id.clone(), now)
        .await
        .map_err(|_| EntryError::InternalServerError)?;

    let res: GetOnchainEntryResponse = GetOnchainEntryResponse {
        pair_id,
        last_updated_timestamp: onchain_pair_median.last_updated_timestamp,
        price: onchain_pair_median.price.to_string(),
        decimals: onchain_pair_median.decimals as u32,
        nb_sources_aggregated: onchain_pair_median.num_sources_aggregated,
        // TODO(akhercha): Only asset type used for now is Crypto
        asset_type: "Crypto".to_string(),
        components: pair_components,
    };
    Ok(Json(res))
}
