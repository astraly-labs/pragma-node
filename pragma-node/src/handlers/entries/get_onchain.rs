use axum::Json;

use axum::extract::{Query, State};

use crate::handlers::entries::{AggregationMode, GetOnchainEntryResponse};
use crate::utils::PathExtractor;
use crate::AppState;
use pragma_entities::EntryError;

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
    State(_state): State<AppState>,
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

    let _agg_mode = if let Some(aggregation_mode) = params.aggregation {
        aggregation_mode
    } else {
        AggregationMode::Twap
    };

    let res = GetOnchainEntryResponse {
        pair_id,
        last_updated_timestamp: now,
        price: "0".to_string(),
        decimals: 8,
        nb_sources_aggregated: 1,
        asset_type: "Crypto".to_string(),
        components: vec![],
    };
    Ok(Json(res))
}
