use axum::{extract::State, Json};
use pragma_entities::EntryError;

use crate::{
    infra::repositories::funding_rates_repository::{self, InstrumentInfo},
    state::AppState,
};

pub type GetSupportedInstrumentsResponse = Vec<InstrumentInfo>;

#[utoipa::path(
    get,
    path = "/node/v1/funding_rates/instruments",
    tag = "Funding Rates",
    responses(
        (status = 200, description = "List of supported instruments and their timestamp range", body = [InstrumentInfo]),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_supported_instruments(
    State(state): State<AppState>,
) -> Result<Json<GetSupportedInstrumentsResponse>, EntryError> {
    funding_rates_repository::get_supported_instruments(&state.offchain_pool)
        .await
        .map(Json)
        .map_err(EntryError::from)
}
