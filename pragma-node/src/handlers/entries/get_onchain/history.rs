use axum::extract::{Query, State};
use axum::Json;
use bigdecimal::BigDecimal;
use pragma_common::types::{AggregationMode, Network};
use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::infra::repositories::onchain_repository::routing;
use crate::types::timestamp::TimestampParam;
use crate::utils::{big_decimal_price_to_hex, PathExtractor};
use crate::AppState;

use super::OnchainEntry;
use crate::utils::currency_pair_to_pair_id;

#[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
pub struct GetOnchainHistoryParams {
    pub network: Network,
    pub aggregation: Option<AggregationMode>,
    pub routing: Option<bool>,
    pub timestamp: Option<TimestampParam>,
    // TODO(akhercha): add block/block_range
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetOnchainHistoryEntry {
    pair_id: String,
    timestamp: u64,
    price: String,
    decimals: u32,
    nb_sources_aggregated: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetOnchainHistoryResponse(pub Vec<GetOnchainHistoryEntry>);

#[utoipa::path(
    get,
    path = "/node/v1/onchain/history/{base}/{quote}",
    responses(
        (status = 200, description = "Get the onchain price history", body = GetOnchainHistoryResponse)
    ),
    params(
        ("base" = String, Path, description = "Base Asset"),
        ("quote" = String, Path, description = "Quote Asset"),
        ("network" = Network, Query, description = "Network"),
        ("aggregation" = Option<AggregationMode>, Query, description = "Aggregation Mode"),
        ("timestamp" = Option<String>, Query, description = "Timestamp or timestamp range")
    ),
)]
pub async fn get_onchain_history(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetOnchainHistoryParams>,
) -> Result<Json<GetOnchainHistoryResponse>, EntryError> {
    tracing::info!("Received get onchain history request for pair {:?}", pair);
    let pair_id: String = currency_pair_to_pair_id(&pair.0, &pair.1);
    let aggregation_mode = params.aggregation.unwrap_or_default();
    let is_routing = params.routing.unwrap_or(false);

    let now = chrono::Utc::now().timestamp();
    let timestamp = params
        .timestamp
        .unwrap_or_else(|| TimestampParam::from(now));
    timestamp.validate_time()?;

    let raw_data = routing(
        &state.onchain_pool,
        &state.offchain_pool,
        params.network,
        pair_id.clone(),
        timestamp.clone(),
        aggregation_mode,
        is_routing,
    )
    .await
    .map_err(|db_error| db_error.to_entry_error(&pair_id))?;

    let mut api_result = Vec::with_capacity(raw_data.len());

    for entry in raw_data {
        api_result.push(adapt_entries_to_onchain_history_entry(
            pair_id.clone(),
            entry.decimal,
            entry.sources,
            0,
            entry.price,
        ))
    }
    Ok(Json(GetOnchainHistoryResponse(api_result)))
}

fn adapt_entries_to_onchain_history_entry(
    pair_id: String,
    decimals: u32,
    sources: Vec<OnchainEntry>,
    timestamp: u64,
    aggregated_price: BigDecimal,
) -> GetOnchainHistoryEntry {
    GetOnchainHistoryEntry {
        pair_id,
        timestamp,
        price: big_decimal_price_to_hex(&aggregated_price),
        decimals,
        nb_sources_aggregated: sources.len() as u32,
    }
}
