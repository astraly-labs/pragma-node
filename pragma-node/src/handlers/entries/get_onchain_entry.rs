use axum::Json;

use axum::extract::State;

use crate::handlers::entries::GetOnchainEntryResponse;
use crate::utils::PathExtractor;
use crate::AppState;
use pragma_entities::EntryError;

use super::utils::currency_pair_to_pair_id;

// TODO: Atm we only retrieve the most recent entry for the given pair
#[utoipa::path(
    get,
    path = "/node/v1/onchain/{base}/{quote}",
    responses(
        (status = 200, description = "Get the latest onchain entry", body = [GetOnchainResponse])
    ),
    params(
        ("base" = String, Path, description = "Base Asset"),
        ("quote" = String, Path, description = "Quote Asset"),
    ),
)]
pub async fn get_onchain_entry(
    State(_state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
) -> Result<Json<GetOnchainEntryResponse>, EntryError> {
    tracing::info!("Received get onchain entry request for pair {:?}", pair);
    let pair_id = currency_pair_to_pair_id(&pair.0, &pair.1);
    let timestamp = chrono::Utc::now().naive_utc().and_utc().timestamp_millis() as u64;

    let res = GetOnchainEntryResponse {
        pair_id,
        price: "0".to_string(),
        timestamp,
        decimals: 0,
        chain: "starknet-sepolia".to_string(),
        publisher: "pragma".to_string(),
        source: "binance".to_string(),
    };
    Ok(Json(res))
}
