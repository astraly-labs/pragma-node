pub mod checkpoints;
pub mod history;
pub mod ohlc;
pub mod publishers;

use std::collections::HashMap;

use axum::extract::{Query, State};
use axum::Json;
use bigdecimal::BigDecimal;
use pragma_common::types::Interval;
use pragma_entities::EntryError;

use crate::handlers::entries::{GetOnchainParams, GetOnchainResponse};
use crate::infra::repositories::onchain_repository::{
    get_last_updated_timestamp, get_variations, routing,
};
use crate::types::timestamp::TimestampParam;
use crate::utils::{big_decimal_price_to_hex, PathExtractor};
use crate::AppState;

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
) -> Result<Json<GetOnchainResponse>, EntryError> {
    tracing::info!("Received get onchain entry request for pair {:?}", pair);
    let pair_id: String = currency_pair_to_pair_id(&pair.0, &pair.1);
    let aggregation_mode = params.aggregation.unwrap_or_default();
    let is_routing = params.routing.unwrap_or(false);
    let with_components = params.components.unwrap_or(true);

    let now = chrono::Utc::now().timestamp();
    let timestamp = params
        .timestamp
        .unwrap_or_else(|| TimestampParam::from(now));
    // NOTE: Only timestamps works for the get_onchain request, not ranges.
    if !timestamp.is_single() {
        return Err(EntryError::InvalidTimestamp(
            "Expected a single timestamp, not a Range.".into(),
        ));
    }
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

    let entry = raw_data
        .first()
        .ok_or_else(|| EntryError::NotFound(pair_id.to_string()))?;

    let last_updated_timestamp =
        get_last_updated_timestamp(&state.onchain_pool, params.network, entry.pair_used.clone())
            .await
            .map_err(|db_error| db_error.to_entry_error(&pair_id))?;

    let variations = get_variations(&state.onchain_pool, params.network, pair_id.clone())
        .await
        .map_err(|db_error| db_error.to_entry_error(&pair_id))?;

    Ok(Json(adapt_entries_to_onchain_response(
        pair_id.clone(),
        entry.decimal,
        entry.sources.clone(),
        entry.price.clone(),
        last_updated_timestamp,
        variations,
        with_components,
    )))
}

fn adapt_entries_to_onchain_response(
    pair_id: String,
    decimals: u32,
    sources: Vec<OnchainEntry>,
    aggregated_price: BigDecimal,
    last_updated_timestamp: u64,
    variations: HashMap<Interval, f32>,
    with_components: bool,
) -> GetOnchainResponse {
    GetOnchainResponse {
        pair_id,
        last_updated_timestamp,
        price: big_decimal_price_to_hex(&aggregated_price),
        decimals,
        nb_sources_aggregated: sources.len() as u32,
        // Only asset type used for now is Crypto
        asset_type: "Crypto".to_string(),
        components: with_components.then_some(sources),
        variations: Some(variations),
    }
}
