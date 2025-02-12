use crate::{infra::repositories::entry_repository, utils::PathExtractor, AppState};
use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
};
use futures::{
    stream::{self, Stream},
    Future,
};
use pragma_common::types::pair::Pair;
use pragma_entities::EntryError;
use std::{convert::Infallible, pin::Pin, time::Duration};
use tokio_stream::StreamExt;

use super::{
    get_entry::{adapt_entry_to_entry_response, GetEntryResponse, RoutingParams},
    GetEntryParams,
};

type BoxedFuture = Pin<Box<dyn Future<Output = Event> + Send>>;
type BoxedStreamItem = Box<dyn FnMut() -> BoxedFuture + Send>;

pub async fn stream_entry(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetEntryParams>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let pair = Pair::from(pair);
    let is_routing = params.routing.unwrap_or(false);

    let interval = params.interval.unwrap_or_default();

    let generator: BoxedStreamItem = match RoutingParams::try_from(params) {
        Ok(routing_params) => {
            let state = state.clone();
            let pair = pair.clone();
            Box::new(move || {
                let state = state.clone();
                let pair = pair.clone();
                let params = routing_params.clone();
                Box::pin(async move {
                    match get_latest_entry(&state, &pair, is_routing, &params).await {
                        Ok(entry_response) => match serde_json::to_string(&entry_response) {
                            Ok(json) => Event::default().data(json),
                            Err(e) => Event::default().data(format!("Serialization error: {}", e)),
                        },
                        Err(e) => Event::default().data(format!("Error fetching entry: {}", e)),
                    }
                }) as BoxedFuture
            })
        }
        Err(e) => {
            let error_message = format!("Error: {}", e);
            Box::new(move || {
                let msg = error_message.clone();
                Box::pin(async move { Event::default().data(msg) }) as BoxedFuture
            })
        }
    };

    let stream = stream::repeat_with(generator)
        .then(|future| future)
        .map(Ok)
        .throttle(Duration::from_secs(interval.to_seconds() as u64));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(60))
            .text("keep-alive-text"),
    )
}

async fn get_latest_entry(
    state: &AppState,
    pair: &Pair,
    is_routing: bool,
    routing_params: &RoutingParams,
) -> Result<GetEntryResponse, EntryError> {
    let (entry, decimals) = entry_repository::routing(
        &state.offchain_pool,
        is_routing,
        pair,
        routing_params.clone(),
    )
    .await
    .map_err(|e| e.to_entry_error(&(pair.to_pair_id())))?;

    let last_updated_timestamp =
        entry_repository::get_last_updated_timestamp(&state.offchain_pool, pair.to_pair_id())
            .await?
            .unwrap_or(entry.time);

    Ok(adapt_entry_to_entry_response(
        pair.to_pair_id(),
        &entry,
        decimals,
        last_updated_timestamp,
    ))
}
