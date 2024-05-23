use axum::Json;

use axum::extract::{Query, State};

use crate::handlers::entries::OnchainEntry;
use crate::handlers::entries::{AggregationMode, GetOnchainEntryResponse};
use crate::infra::repositories::onchain_repository;
use crate::utils::PathExtractor;
use crate::AppState;
use pragma_entities::{error::InfraError, EntryError};

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

    let _agg_mode = if let Some(aggregation_mode) = params.aggregation {
        aggregation_mode
    } else {
        AggregationMode::Twap
    };

    let latest_spot_entry =
        onchain_repository::get_latest_spot(&state.onchain_pool, pair_id.clone())
            .await
            .map_err(|e: InfraError| to_entry_error(e, &pair_id))?;
    let res: GetOnchainEntryResponse = GetOnchainEntryResponse {
        pair_id: latest_spot_entry.pair_id,
        last_updated_timestamp: latest_spot_entry.timestamp.and_utc().timestamp() as u64,
        price: latest_spot_entry.price.to_string(),
        decimals: 8,
        nb_sources_aggregated: 1,
        asset_type: "Crypto".to_string(),
        components: vec![OnchainEntry {
            publisher: latest_spot_entry.publisher,
            source: latest_spot_entry.source,
            price: latest_spot_entry.price.to_string(),
            tx_hash: latest_spot_entry.transaction_hash,
            timestamp: latest_spot_entry.timestamp.and_utc().timestamp() as u64,
        }],
    };
    Ok(Json(res))
}

pub(crate) fn to_entry_error(error: InfraError, pair_id: &String) -> EntryError {
    match error {
        InfraError::InternalServerError => EntryError::InternalServerError,
        InfraError::NotFound => EntryError::NotFound(pair_id.to_string()),
        InfraError::InvalidTimeStamp => EntryError::InvalidTimestamp,
    }
}
