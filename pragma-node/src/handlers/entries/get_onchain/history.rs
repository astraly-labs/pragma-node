use std::collections::HashSet;

use axum::extract::{Query, State};
use axum::Json;
use bigdecimal::BigDecimal;
use pragma_common::types::{AggregationMode, Network};
use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::infra::repositories::onchain_repository::{routing, RawOnchainData};
use crate::types::timestamp::TimestampParam;
use crate::utils::{big_decimal_price_to_hex, PathExtractor};
use crate::AppState;

use super::OnchainEntry;
use crate::utils::currency_pair_to_pair_id;

#[derive(Default, Debug, Serialize, Deserialize, ToSchema, Clone, Copy)]
pub enum ChunkInterval {
    #[serde(rename = "10s")]
    TenSeconds,
    #[serde(rename = "1min")]
    OneMinute,
    #[serde(rename = "15min")]
    FifteenMinutes,
    #[serde(rename = "30min")]
    ThirtyMinutes,
    #[serde(rename = "1h")]
    #[default]
    OneHour,
    #[serde(rename = "2h")]
    TwoHours,
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "1w")]
    OneWeek,
}

impl ChunkInterval {
    pub fn as_sql_interval(&self) -> &str {
        match self {
            Self::TenSeconds => "10 seconds",
            Self::OneMinute => "1 minute",
            Self::FifteenMinutes => "15 minutes",
            Self::ThirtyMinutes => "30 minutes",
            Self::OneHour => "1 hour",
            Self::TwoHours => "2 hours",
            Self::OneDay => "1 day",
            Self::OneWeek => "1 week",
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
pub struct GetOnchainHistoryParams {
    pub network: Network,
    pub aggregation: Option<AggregationMode>,
    pub timestamp: Option<TimestampParam>,
    // TODO(akhercha): add block/block_range
    pub chunk_interval: Option<ChunkInterval>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetOnchainHistoryEntry {
    pair_id: String,
    last_updated_timestamp: u64,
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
        ("timestamp" = Option<String>, Query, description = "Timestamp or timestamp range"),
        (
            "chunk_interval" = Option<ChunkInterval>,
            Query,
            description = "Chunk time length for each block of data, will always be 1 hour for single timestamps",
        )
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
    let mut chunk_interval = params.chunk_interval.unwrap_or_default();

    let timestamp = params
        .timestamp
        .unwrap_or_default()
        .assert_time_is_valid()?;

    // NOTE: For single timestamps, chunk interval is always [ChunkInterval::OneHour]
    // to align with `get_onchain`.
    if timestamp.is_single() {
        chunk_interval = ChunkInterval::OneHour;
    }

    let raw_data = routing(
        &state.onchain_pool,
        &state.offchain_pool,
        params.network,
        pair_id.clone(),
        timestamp.clone(),
        aggregation_mode,
        false,
        chunk_interval,
    )
    .await
    .map_err(|db_error| db_error.to_entry_error(&pair_id))?;

    let response = prepare_response(&pair_id, raw_data);
    Ok(Json(response))
}

fn prepare_response(pair_id: &str, raw_data: Vec<RawOnchainData>) -> GetOnchainHistoryResponse {
    let mut api_result = Vec::with_capacity(raw_data.len());
    for entry in raw_data {
        api_result.push(adapt_entries_to_onchain_history_entry(
            pair_id.to_owned(),
            entry.decimal,
            entry.sources,
            entry.price,
        ));
    }
    GetOnchainHistoryResponse(api_result)
}

fn adapt_entries_to_onchain_history_entry(
    pair_id: String,
    decimals: u32,
    sources: Vec<OnchainEntry>,
    aggregated_price: BigDecimal,
) -> GetOnchainHistoryEntry {
    let last_updated_timestamp = sources
        .iter()
        .map(|source| source.timestamp)
        .max()
        .unwrap_or(0);

    let nb_sources_aggregated = sources
        .iter()
        .map(|entry| &entry.source)
        .collect::<HashSet<_>>()
        .len() as u32;

    GetOnchainHistoryEntry {
        pair_id,
        last_updated_timestamp,
        price: big_decimal_price_to_hex(&aggregated_price),
        decimals,
        nb_sources_aggregated,
    }
}
