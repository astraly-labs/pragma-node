use std::{convert::Infallible, time::Duration};

use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;

use pragma_common::types::{Interval, Network};
use pragma_entities::InfraError;

use crate::{
    infra::repositories::{entry_repository::OHLCEntry, onchain_repository},
    AppState,
};

#[derive(Debug, Serialize, Deserialize)]
struct OHLCRequest {
    pair: String,
    network: Network,
    interval: Interval,
    candles_to_get: Option<u64>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct OHLCResponse {
    pub pair_id: String,
    pub data: Vec<OHLCEntry>,
}

pub async fn stream_ohlc(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Create a stream that emits OHLC data every 30 seconds
    let stream = stream::repeat_with(move || {
        let state = state.clone();
        async move {
            match get_latest_ohlc(&state).await {
                Ok(ohlc_data) => match serde_json::to_string(&ohlc_data) {
                    Ok(json) => Event::default().data(json),
                    Err(_) => Event::default().data("Error serializing OHLC data"),
                },
                Err(_) => Event::default().data("Error fetching OHLC data"),
            }
        }
    })
    .then(|future| future)
    .map(Ok)
    .throttle(Duration::from_secs(30));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive-text"),
    )
}

async fn get_latest_ohlc(state: &AppState) -> Result<OHLCResponse, InfraError> {
    // TODO: Make those configurable
    let pair_id = "BTC/USD".to_string();
    let network = Network::Mainnet;
    let interval = Interval::OneMinute;
    let candles_to_get = 1;

    let ohlc = onchain_repository::ohlc::get_ohlc(
        &state.onchain_pool,
        network,
        pair_id.clone(),
        interval,
        candles_to_get,
    )
    .await?;

    Ok(OHLCResponse {
        pair_id,
        data: ohlc,
    })
}
