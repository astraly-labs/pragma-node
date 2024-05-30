use axum::extract::{Query, State};
use axum::Json;

use pragma_entities::EntryError;

use crate::handlers::entries::{GetOnchainOHLCParams, GetOnchainOHLCResponse};
use crate::utils::PathExtractor;
use crate::AppState;

use crate::handlers::entries::utils::currency_pair_to_pair_id;

#[utoipa::path(
    get,
    path = "/node/v1/onchain/ohlc/{base}/{quote}",
    responses(
        (status = 200, description = "Get the onchain OHLC data for a pair", body = GetOnchainPublishersResponse)
    ),
    params(
        ("base" = String, Path, description = "Base Asset"),
        ("quote" = String, Path, description = "Quote Asset"),
        ("network" = Network, Query, description = "Network"),
        ("interval" = Interval, Query, description = "Interval of the OHLC data"),
    ),
)]
pub async fn get_onchain_ohlc(
    State(_state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(_params): Query<GetOnchainOHLCParams>,
) -> Result<Json<GetOnchainOHLCResponse>, EntryError> {
    let pair_id = currency_pair_to_pair_id(&pair.0, &pair.1);

    // TODO(akhercha): Get OHLC data for the pair
    let ohlc_data = vec![];

    let r = GetOnchainOHLCResponse {
        pair_id,
        data: ohlc_data,
    };

    Ok(Json(r))
}
