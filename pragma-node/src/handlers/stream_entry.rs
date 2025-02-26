use std::{convert::Infallible, pin::Pin, time::Duration};

use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
};
use axum_extra::{headers, TypedHeader};
use futures::{
    stream::{self, Stream},
    Future,
};
use serde::Deserialize;
use tokio_stream::StreamExt;
use utoipa::{IntoParams, ToSchema};

use pragma_common::types::{pair::Pair, AggregationMode};
use pragma_entities::EntryError;

use super::{
    get_entry::{adapt_entry_to_entry_response, GetEntryResponse, RoutingParams},
    GetEntryParams,
};

use crate::{infra::repositories::entry_repository, utils::PathExtractor, AppState};

const DEFAULT_HISTORICAL_PRICES: usize = 50;

type BoxedFuture = Pin<Box<dyn Future<Output = Event> + Send>>;
type BoxedStreamItem = Box<dyn FnMut() -> BoxedFuture + Send>;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct StreamEntryParams {
    #[serde(flatten)]
    pub get_entry_params: GetEntryParams,
    pub historical_prices: Option<usize>,
}

#[allow(clippy::too_many_lines)]
pub async fn stream_entry(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<StreamEntryParams>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let pair = Pair::from(pair);
    let is_routing = params.get_entry_params.routing.unwrap_or(false);
    let interval = params.get_entry_params.interval.unwrap_or_default();
    let historical_prices = params
        .historical_prices
        .unwrap_or(DEFAULT_HISTORICAL_PRICES);

    tracing::info!(
        "`{}` connected to price feed {} with {} historical prices",
        user_agent.as_str(),
        pair.to_pair_id(),
        historical_prices
    );

    let generator: BoxedStreamItem = if is_routing
        || params.get_entry_params.timestamp.is_some()
        || matches!(
            params.get_entry_params.aggregation,
            Some(AggregationMode::Twap | AggregationMode::Mean)
        ) {
        let mut sent_error = false;
        Box::new(move || {
            let first = !sent_error;
            sent_error = true;

            Box::pin(async move {
                if first {
                    Event::default()
                        .json_data(serde_json::json!({
                            "error": "SSE streaming for entries only works with no routing & for median."
                        }))
                        .unwrap_or_else(|_| Event::default().data(r#"{"error": "Error serializing error message"}"#))
                } else {
                    Event::default()
                }
            })
        })
    } else {
        match RoutingParams::try_from(params.get_entry_params) {
            Ok(get_entry_params) => {
                let mut first_batch = true;

                Box::new(move || {
                    let state = state.clone();
                    let pair = pair.clone();
                    let params = get_entry_params.clone();

                    let is_first = first_batch;
                    first_batch = false;

                    Box::pin(async move {
                        if is_first {
                            // For the first batch, get historical prices
                            match get_historical_entries(&state, &pair, &params, historical_prices).await {
                                Ok(entries) => Event::default()
                                    .json_data(&entries)
                                    .unwrap_or_else(|e| Event::default().json_data(serde_json::json!({
                                        "error": format!("Error serializing historical entries: {e}")
                                    })).unwrap())
                                    .event("historical"),
                                Err(e) => Event::default()
                                    .json_data(serde_json::json!({
                                        "error": format!("Error fetching historical entries: {e}")
                                    }))
                                    .unwrap_or_else(|_| Event::default().data(r#"{"error": "Error serializing error message"}"#)),
                            }
                        } else {
                            // For subsequent updates, get latest price
                            match get_latest_entry(&state, &pair, is_routing, &params).await {
                                Ok(entry_response) => Event::default()
                                    .json_data(&entry_response)
                                    .unwrap_or_else(|e| {
                                        Event::default()
                                            .json_data(serde_json::json!({
                                                "error": format!("Error serializing entry: {e}")
                                            }))
                                            .unwrap()
                                    }),
                                Err(e) => Event::default()
                                    .json_data(serde_json::json!({
                                        "error": format!("Error fetching entry: {e}")
                                    }))
                                    .unwrap_or_else(|_| {
                                        Event::default()
                                            .data(r#"{"error": "Error serializing error message"}"#)
                                    }),
                            }
                        }
                    }) as BoxedFuture
                })
            }
            Err(e) => {
                let error_message = format!("Error: {e}");
                Box::new(move || {
                    let msg = error_message.clone();
                    Box::pin(async move {
                        Event::default()
                            .json_data(serde_json::json!({
                                "error": msg
                            }))
                            .unwrap_or_else(|_| {
                                Event::default()
                                    .data(r#"{"error": "Error serializing error message"}"#)
                            })
                    }) as BoxedFuture
                })
            }
        }
    };

    let stream = stream::repeat_with(generator)
        .then(|future| future)
        .map(Ok)
        .throttle(Duration::from_secs(interval.to_seconds() as u64));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive-text"),
    )
}

async fn get_historical_entries(
    state: &AppState,
    pair: &Pair,
    routing_params: &RoutingParams,
    count: usize,
) -> Result<Vec<GetEntryResponse>, EntryError> {
    let interval = routing_params.interval;
    // Get current timestamp
    let end_timestamp = chrono::Utc::now().timestamp() as u64;
    // Get timestamp from count minutes ago
    let start_timestamp = end_timestamp.saturating_sub(count as u64 * interval.to_seconds() as u64);

    // Get entries based on aggregation mode
    let entries = match routing_params.aggregation_mode {
        AggregationMode::Median => entry_repository::get_median_prices_between(
            &state.offchain_pool,
            pair.to_pair_id(),
            routing_params.clone(),
            start_timestamp,
            end_timestamp,
        )
        .await
        .map_err(|e| e.to_entry_error(&pair.to_pair_id()))?,
        AggregationMode::Mean | AggregationMode::Twap => unreachable!(),
    };

    let responses: Vec<GetEntryResponse> = entries
        .into_iter()
        .take(count)
        .map(|entry| adapt_entry_to_entry_response(pair.to_pair_id(), &entry, entry.time))
        .collect();

    Ok(responses)
}

async fn get_latest_entry(
    state: &AppState,
    pair: &Pair,
    is_routing: bool,
    routing_params: &RoutingParams,
) -> Result<GetEntryResponse, EntryError> {
    // We have to update the timestamp to now every tick
    let mut new_routing = routing_params.clone();
    new_routing.timestamp = chrono::Utc::now().timestamp();

    let entry = entry_repository::routing(&state.offchain_pool, is_routing, pair, &new_routing)
        .await
        .map_err(|e| e.to_entry_error(&(pair.to_pair_id())))?;

    let last_updated_timestamp = entry_repository::get_last_updated_timestamp(
        &state.offchain_pool,
        pair.to_pair_id(),
        new_routing.timestamp,
    )
    .await?
    .unwrap_or(entry.time);

    Ok(adapt_entry_to_entry_response(
        pair.to_pair_id(),
        &entry,
        last_updated_timestamp,
    ))
}
