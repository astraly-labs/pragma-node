use axum::extract::{Query, State};
use axum::Json;

use pragma_entities::EntryError;

use crate::handlers::entries::{AggregationMode, GetOnchainEntryResponse};
use crate::infra::onchain::get_data_median;
use crate::utils::PathExtractor;
use crate::AppState;

use super::utils::currency_pair_to_pair_id;
use super::GetOnchainParams;

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
    let _timestamp = if let Some(timestamp) = params.timestamp {
        timestamp
    } else {
        now
    };

    // TODO(akhercha): Currently only agg_mode used is Median
    let _agg_mode = if let Some(aggregation_mode) = params.aggregation {
        aggregation_mode
    } else {
        AggregationMode::Median
    };

    // TODO(akhercha): Call `get_data` with correct parameters
    let onchain_pair_median: crate::infra::onchain::GetDataMedianResponse =
        get_data_median(state.network.clone(), pair_id.clone())
            .await
            .map_err(|e| {
                tracing::error!("Failed to get onchain data: {:?}", e);
                EntryError::InternalServerError
            })?;

    let res: GetOnchainEntryResponse = GetOnchainEntryResponse {
        pair_id: pair_id,
        last_updated_timestamp: onchain_pair_median.last_updated_timestamp,
        price: onchain_pair_median.price.to_string(),
        decimals: onchain_pair_median.decimals as u32,
        nb_sources_aggregated: onchain_pair_median.num_sources_aggregated,
        // The only asset handled is Crypto for now
        asset_type: "Crypto".to_string(),
    };
    Ok(Json(res))
}
