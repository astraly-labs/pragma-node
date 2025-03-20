use std::{convert::Infallible, time::Duration};

use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
};
use axum_extra::{TypedHeader, headers};
use futures::stream::{self, Stream};
use serde::Deserialize;
use tokio_stream::StreamExt;
use utoipa::{IntoParams, ToSchema};

use pragma_common::types::{AggregationMode, Interval, pair::Pair};

use crate::{
    AppState,
    handlers::{
        GetEntryParams,
        get_entry::EntryParams,
        stream::{
            BoxedFuture, BoxedStreamItem, DEFAULT_HISTORICAL_PRICES, get_historical_entries,
            get_latest_entry,
        },
    },
    utils::PathExtractor,
};

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
    let interval = params
        .get_entry_params
        .interval
        .unwrap_or(Interval::OneHundredMillisecond);
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
            Some(AggregationMode::Twap)
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
        match EntryParams::try_from(params.get_entry_params) {
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
                            match get_latest_entry(&state, &pair, is_routing, &params, false).await
                            {
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
        .throttle(interval.into());

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive-text"),
    )
}
