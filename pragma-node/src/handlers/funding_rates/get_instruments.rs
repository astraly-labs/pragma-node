use std::collections::HashMap;

use axum::{Json, extract::State};
use pragma_entities::EntryError;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    infra::repositories::funding_rates_repository::{self, InstrumentInfo},
    state::AppState,
};

pub type GetSupportedInstrumentsResponse = Vec<InstrumentInfo>;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentDto {
    pub instrument_id: String,
    pub first_timestamp_ms: u64,
    pub last_timestamp_ms: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SupportedInstrumentsResponse {
    #[serde(flatten)]
    pub data: HashMap<String, Vec<InstrumentDto>>,
}

#[utoipa::path(
    get,
    path = "/node/v1/funding_rates/instruments",
    tag = "Funding Rates",
    responses(
        (status = 200, body = SupportedInstrumentsResponse, description = "Success"),
        (status = 500, description = "Server error")
    )
)]
pub async fn get_supported_instruments(
    State(state): State<AppState>,
) -> Result<Json<SupportedInstrumentsResponse>, EntryError> {
    let instruments = funding_rates_repository::get_supported_instruments(&state.offchain_pool)
        .await
        .map_err(EntryError::from)?;

    // Regroupe simplement par exchange (`source`)
    let mut data: HashMap<String, Vec<InstrumentDto>> = HashMap::new();
    for info in instruments {
        data.entry(info.source).or_default().push(InstrumentDto {
            instrument_id: info.pair,
            first_timestamp_ms: info.first_timestamp_ms,
            last_timestamp_ms: info.last_timestamp_ms,
        });
    }

    Ok(Json(SupportedInstrumentsResponse { data }))
}
