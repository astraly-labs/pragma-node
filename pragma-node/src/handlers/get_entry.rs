use axum::Json;
use axum::extract::{Query, State};
use chrono::{DateTime, NaiveDateTime, Utc};
use pragma_entities::models::entries::timestamp::TimestampRangeError;
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

use pragma_common::Pair;
use pragma_common::{AggregationMode, InstrumentType, Interval};
use pragma_entities::{EntryError, TimestampError};

use crate::constants::EIGHTEEN_DECIMALS;
use crate::infra::repositories::entry_repository::{
    MedianEntry, get_last_updated_timestamp, routing,
};
use crate::state::AppState;
use crate::utils::PathExtractor;
use crate::utils::big_decimal_price_to_hex;

use super::GetEntryParams;

#[derive(Default, Clone, Debug)]
pub struct EntryParams {
    pub interval: Interval,
    pub timestamp: i64,
    pub aggregation_mode: AggregationMode,
    pub data_type: InstrumentType,
    pub expiry: String,
    pub with_components: bool,
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
        let with_components = params.with_components.unwrap_or(false);

        // Convert entry_type to DataType
        let data_type = params
            .entry_type
            .map_or(InstrumentType::Spot, InstrumentType::from);

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
            with_components,
        })
    }
}
#[derive(Debug, Serialize, Deserialize, ToSchema, ToResponse)]
pub struct GetEntryResponse {
    pub num_sources_aggregated: usize,
    pub pair_id: String,
    pub price: String,
    pub timestamp: u64,
    pub decimals: u32,
    pub components: Option<Vec<EntryComponent>>,
}

/// Get the latest price entry for a trading pair
#[utoipa::path(
    get,
    path = "/node/v1/data/{base}/{quote}",
    tag = "Price Data",
    responses(
        (status = 200, description = "Successfully retrieved price entry", body = GetEntryResponse,
         example = json!({
            "num_sources_aggregated": 5,
            "pair_id": "BTC/USD",
            "price": "0x1234567890abcdef",
            "timestamp": 1_647_820_800,
            "decimals": 18, 
            "components": [
                {
                    "source": "BINANCE",
                    "price": "0x6cc61113f5871b1000",
                    "timestamp": 1_743_082_057
                },
            ]
         })
        ),
        (status = 400, description = "Invalid request parameters", body = EntryError),
        (status = 404, description = "Price entry not found", body = EntryError),
        (status = 500, description = "Internal server error", body = EntryError)
    ),
    params(
        ("base" = String, Path, description = "Base asset symbol (e.g. BTC)"),
        ("quote" = String, Path, description = "Quote asset symbol (e.g. USD)"),
        GetEntryParams
    ),
    security(
        ("api_key" = [])
    )
)]
#[tracing::instrument(skip(state))]
pub async fn get_entry(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetEntryParams>,
) -> Result<Json<GetEntryResponse>, EntryError> {
    let is_routing = params.routing.unwrap_or(true);

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
        decimals: EIGHTEEN_DECIMALS,
        components: entry
            .components
            .as_ref()
            .map(|prices| prices.iter().cloned().map(Into::into).collect()),
    }
}

#[derive(Serialize, Deserialize, Default, ToSchema, Clone, ToResponse, Debug)]
pub struct EntryComponent {
    pub source: String,
    pub price: String,
    pub timestamp: u64,
}
