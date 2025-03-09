use axum::Json;
use axum::extract::{Query, State};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

use pragma_common::timestamp::{TimestampError, TimestampRangeError};
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
pub struct EntryParams {
    pub interval: Interval,
    pub timestamp: i64,
    pub aggregation_mode: AggregationMode,
    pub data_type: DataType,
    pub expiry: String,
}

impl TryFrom<GetEntryParams> for EntryParams {
    type Error = EntryError;

    fn try_from(params: GetEntryParams) -> Result<Self, Self::Error> {
        let now = chrono::Utc::now().timestamp();

        // Unwrap timestamp or use current time
        let timestamp = params.timestamp.unwrap_or(now);

        // Validate timestamp isn't in the future
        if timestamp > now {
            return Err(EntryError::InvalidTimestamp(TimestampError::RangeError(
                TimestampRangeError::EndInFuture,
            )));
        }

        // Unwrap parameters with their defaults
        let interval = params.interval.unwrap_or_default();
        let aggregation_mode = params.aggregation.unwrap_or_default();

        // Validate TWAP aggregation constraints
        // NOTE: Fixed logic error in the original condition
        if matches!(aggregation_mode, AggregationMode::Twap)
            && interval != Interval::OneHour
            && interval != Interval::TwoHours
        {
            return Err(EntryError::InvalidInterval(interval, aggregation_mode));
        }

        // Convert entry_type to DataType
        let data_type = params
            .entry_type
            .map_or(DataType::SpotEntry, DataType::from);

        // Parse and format expiry date if provided
        let expiry = match params.expiry {
            Some(expiry_str) => NaiveDateTime::parse_from_str(&expiry_str, "%Y-%m-%dT%H:%M:%S")
                .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S%:z").to_string())
                .map_err(|_| EntryError::InvalidExpiry)?,
            None => String::default(),
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

    let entry_params = EntryParams::try_from(params)?;

    let pair = Pair::from(pair);

    let entry = routing(&state.offchain_pool, is_routing, &pair, &entry_params)
        .await
        .map_err(EntryError::from)?;

    let last_updated_timestamp: NaiveDateTime = get_last_updated_timestamp(
        &state.offchain_pool,
        pair.to_pair_id(),
        entry_params.timestamp,
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
