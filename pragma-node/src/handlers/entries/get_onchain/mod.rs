pub mod checkpoints;
pub mod ohlc;
pub mod publishers;

use axum::extract::{Query, State};
use axum::Json;
use bigdecimal::BigDecimal;
use pragma_entities::EntryError;

use crate::handlers::entries::{GetOnchainParams, GetOnchainResponse};
use crate::infra::repositories::onchain_repository::{get_last_updated_timestamp, routing};
use crate::utils::{big_decimal_price_to_hex, PathExtractor};
use crate::{handlers, AppState};

use super::OnchainEntry;
use crate::utils::currency_pair_to_pair_id;

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
        ("aggregation" = Option<AggregationMode>, Query, description = "Aggregation Mode"),
        ("timestamp" = Option<u64>, Query, description = "Timestamp")
    ),
)]
pub async fn get_onchain(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetOnchainParams>,
) -> Result<Json<Vec<GetOnchainResponse>>, EntryError> {
    tracing::info!("Received get onchain entry request for pair {:?}", pair);
    let is_routing = params.routing.unwrap_or(false);

    let pair_id: String = currency_pair_to_pair_id(&pair.0, &pair.1);
    let now = chrono::Utc::now().timestamp() as u64;
    let aggregation_mode = params.aggregation.unwrap_or_default();
    let timestamp = match params.timestamp {
        Some(handlers::entries::TimestampParam::Single(ts)) => {
            if ts > now {
                return Err(EntryError::InvalidTimestamp);
            }
            handlers::entries::TimestampParam::Single(ts)
        }
        Some(handlers::entries::TimestampParam::Range(range)) => {
            // Check if start is after end
            if range.start() > range.end() {
                return Err(EntryError::InvalidTimestamp);
            }

            // Check if end is in the future
            if *range.end() > now {
                return Err(EntryError::InvalidTimestamp);
            }
            handlers::entries::TimestampParam::Range(range)
        }
        None => handlers::entries::TimestampParam::Single(now),
    };

    let raw_data = routing(
        &state.onchain_pool,
        &state.offchain_pool,
        params.network,
        pair_id.clone(),
        timestamp,
        aggregation_mode,
        is_routing,
    )
    .await
    .map_err(|db_error| db_error.to_entry_error(&pair_id))?;

    // TODO(akhercha): ⚠ gives different result than onchain oracle sometime
    let last_updated_timestamp = get_last_updated_timestamp(
        &state.onchain_pool,
        params.network,
        raw_data[0].pair_used.clone(),
    )
    .await
    .map_err(|db_error| db_error.to_entry_error(&pair_id))?;

    let mut api_result: Vec<GetOnchainResponse> = Vec::with_capacity(raw_data.len());
    for entries in raw_data {
        api_result.push(adapt_entries_to_onchain_response(
            pair_id.clone(),
            entries.decimal,
            entries.sources,
            entries.price,
            last_updated_timestamp,
        ));
    }
    Ok(Json(api_result))
}

fn adapt_entries_to_onchain_response(
    pair_id: String,
    decimals: u32,
    sources: Vec<OnchainEntry>,
    aggregated_price: BigDecimal,
    last_updated_timestamp: u64,
) -> GetOnchainResponse {
    GetOnchainResponse {
        pair_id,
        last_updated_timestamp,
        price: big_decimal_price_to_hex(&aggregated_price),
        decimals,
        nb_sources_aggregated: sources.len() as u32,
        // Only asset type used for now is Crypto
        asset_type: "Crypto".to_string(),
        components: sources,
    }
}
