use axum::Json;
use axum::extract::{Query, State};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

use pragma_common::timestamp::TimestampRangeError;
use pragma_common::types::pair::Pair;
use pragma_common::types::{AggregationMode, DataType, Interval};
use pragma_entities::EntryError;

use crate::AppState;
use crate::constants::PRAGMA_DECIMALS;
use crate::infra::repositories::entry_repository::{
    MedianEntry, get_last_updated_timestamp, routing,
};
use crate::utils::PathExtractor;
use crate::utils::big_decimal_price_to_hex;

use super::GetEntryParams;

#[derive(Default, Clone, Debug)]
pub struct RoutingParams {
    pub interval: Interval,
    pub timestamp: i64,
    pub aggregation_mode: AggregationMode,
    pub data_type: DataType,
    pub expiry: String,
}

impl TryFrom<GetEntryParams> for RoutingParams {
    type Error = EntryError;

    fn try_from(params: GetEntryParams) -> Result<Self, Self::Error> {
        let now = chrono::Utc::now().timestamp();

        let timestamp = params.timestamp.map_or(now, |timestamp| timestamp);

        if timestamp > now {
            return Err(EntryError::InvalidTimestamp(
                TimestampRangeError::EndInFuture,
            ));
        }

        let interval = params
            .interval
            .map_or(Interval::TwoHours, |interval| interval);

        let aggregation_mode = params
            .aggregation
            .map_or(AggregationMode::Twap, |aggregation_mode| aggregation_mode);

        let data_type = params
            .entry_type
            .map_or(DataType::SpotEntry, DataType::from);

        let expiry = if let Some(expiry) = params.expiry {
            let expiry_dt = NaiveDateTime::parse_from_str(&expiry, "%Y-%m-%dT%H:%M:%S")
                .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
            match expiry_dt {
                Ok(expiry_dt) => expiry_dt.format("%Y-%m-%d %H:%M:%S%:z").to_string(),
                Err(_) => return Err(EntryError::InvalidExpiry),
            }
        } else {
            String::default()
        };

        Ok(Self {
            interval,
            timestamp,
            aggregation_mode,
            data_type,
            expiry,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, ToResponse)]
pub struct GetEntryResponse {
    num_sources_aggregated: usize,
    pair_id: String,
    price: String,
    timestamp: u64,
    decimals: u32,
}

#[utoipa::path(
    get,
    path = "/node/v1/data/{base}/{quote}",
    responses(
        (status = 200, description = "Get median entry successfuly", body = [GetEntryResponse])
    ),
    params(
        ("base" = String, Path, description = "Base Asset"),
        ("quote" = String, Path, description = "Quote Asset"),
        GetEntryParams,
    ),
)]
#[tracing::instrument(skip(state))]
pub async fn get_entry(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetEntryParams>,
) -> Result<Json<GetEntryResponse>, EntryError> {
    let is_routing = params.routing.unwrap_or(false);

    let routing_params = RoutingParams::try_from(params)?;

    let pair = Pair::from(pair);

    let entry = routing(&state.offchain_pool, is_routing, &pair, &routing_params)
        .await
        .map_err(|e| e.to_entry_error(&(pair.to_pair_id())))?;

    let last_updated_timestamp: NaiveDateTime = get_last_updated_timestamp(
        &state.offchain_pool,
        pair.to_pair_id(),
        routing_params.timestamp,
    )
    .await?
    .unwrap_or(entry.time);

    Ok(Json(adapt_entry_to_entry_response(
        pair.into(),
        &entry,
        last_updated_timestamp,
    )))
}

pub fn adapt_entry_to_entry_response(
    pair_id: String,
    entry: &MedianEntry,
    last_updated_timestamp: NaiveDateTime,
) -> GetEntryResponse {
    GetEntryResponse {
        pair_id,
        timestamp: last_updated_timestamp.and_utc().timestamp_millis() as u64,
        num_sources_aggregated: entry.num_sources as usize,
        price: big_decimal_price_to_hex(&entry.median_price),
        decimals: PRAGMA_DECIMALS,
    }
}
