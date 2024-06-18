use std::str::FromStr;

use axum::extract::State;
use axum::Json;
use bigdecimal::num_bigint::{BigInt, ToBigInt};
use bigdecimal::BigDecimal;

use crate::infra::errors::InfraError;
use crate::infra::repositories::entry_repository::{self, MedianEntry};
use crate::utils::PathExtractor;
use crate::AppState;
use pragma_entities::EntryError;

use crate::utils::compute_median_price_and_time;
use super::ConvertAmountResponse;

/// Converts a currency pair to a pair id.
fn currency_pair_to_pair_id(quote: &str, base: &str) -> String {
    format!("{}/{}", quote.to_uppercase(), base.to_uppercase())
}

#[utoipa::path(
        get,
        path = "/node/v1/data/{quote}/{base}/{amount}",
        responses(
            (status = 200, description = "Amount converted successfuly", body = [ConvertAmountResponse])
        ),
        params(
            ("quote" = String, Path, description = "Quote Asset"),
            ("base" = String, Path, description = "Base Asset"),
            ("amount" = String, Path, description = "Amount of base asset to convert")
        ),
    )]
pub async fn convert_amount(
    State(state): State<AppState>,
    PathExtractor(input): PathExtractor<(String, String, String)>,
) -> Result<Json<ConvertAmountResponse>, EntryError> {
    tracing::info!("Received convert amount request with input {:?}", input);
    // Construct pair id
    let pair_id = currency_pair_to_pair_id(&input.0, &input.1);

    // Parse amount
    let amount = BigDecimal::from_str(&input.2).map_err(|_| EntryError::InvalidAmount(input.2))?;

    if pair_id == "STRK/ETH" {
        let price = BigDecimal::from_str("100000000000000000").unwrap();
        let decimals = 18;

        let converted_amount = amount / price.clone();
        let scaler = BigInt::from(10).pow(decimals);
        let converted_amount = converted_amount
            .to_bigint()
            .unwrap()
            .checked_mul(&scaler)
            .unwrap();

        return Ok(Json(ConvertAmountResponse {
            pair_id,
            timestamp: chrono::Utc::now().timestamp() as u64,
            num_sources_aggregated: 5,
            price: "16345785D8A0000".to_string(), // 0.1 wei
            converted_amount: converted_amount.to_str_radix(16),
        }));
    }

    // Get entries from database with given pair id (only the latest one grouped by publisher)
    let mut entries = entry_repository::get_median_entries(&state.pool, pair_id.clone())
        .await
        .map_err(|db_error| match db_error {
            InfraError::InternalServerError => EntryError::InternalServerError,
            InfraError::NotFound => EntryError::NotFound(pair_id.clone()),
        })?;

    let decimals = entry_repository::get_decimals(&state.pool, &pair_id)
        .await
        .map_err(|db_error| match db_error {
            InfraError::InternalServerError => EntryError::InternalServerError,
            InfraError::NotFound => EntryError::NotFound(pair_id.clone()),
        })?;

    Ok(Json(adapt_entries_to_convert_response(
        pair_id,
        &mut entries,
        amount,
        decimals,
    )))
}

fn adapt_entries_to_convert_response(
    pair_id: String,
    entries: &mut Vec<MedianEntry>,
    amount: BigDecimal,
    decimals: u32,
) -> ConvertAmountResponse {
    let (price, timestamp) = compute_median_price_and_time(entries).unwrap_or_default();

    let converted_amount = amount / price.clone();
    let scaler = BigInt::from(10).pow(decimals);
    let converted_amount = converted_amount
        .to_bigint()
        .unwrap()
        .checked_mul(&scaler)
        .unwrap();

    ConvertAmountResponse {
        pair_id,
        timestamp: timestamp.timestamp() as u64,
        num_sources_aggregated: entries.len(),
        price: price.to_bigint().unwrap().to_str_radix(16),
        converted_amount: converted_amount.to_str_radix(16),
    }
}
