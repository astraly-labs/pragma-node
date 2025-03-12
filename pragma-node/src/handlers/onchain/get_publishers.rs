use axum::Json;
use axum::extract::{Query, State};

use pragma_common::types::{DataType, Network};
use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToResponse, ToSchema};

use crate::AppState;
use crate::infra::repositories::onchain_repository::publisher::{
    get_publishers, get_publishers_with_components,
};

#[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
pub struct GetOnchainPublishersParams {
    pub network: Network,
    pub data_type: DataType,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PublisherEntry {
    pub pair_id: String,
    pub last_updated_timestamp: u64,
    pub price: String,
    pub source: String,
    pub decimals: u32,
    pub daily_updates: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Publisher {
    pub publisher: String,
    pub website_url: String,
    pub last_updated_timestamp: Option<u64>,
    pub r#type: u32,
    pub nb_feeds: u32,
    pub daily_updates: u32,
    pub total_updates: u32,
    pub components: Vec<PublisherEntry>,
}

#[derive(Debug, Default, Serialize, Deserialize, ToResponse, ToSchema)]
pub struct GetOnchainPublishersResponse(pub Vec<Publisher>);

#[utoipa::path(
    get,
    path = "/node/v1/onchain/publishers",
    responses(
        (status = 200, description = "Get the onchain publishers", body = GetOnchainPublishersResponse)
    ),
    params(
       GetOnchainPublishersParams
    ),
)]
#[tracing::instrument(skip(state))]
pub async fn get_onchain_publishers(
    State(state): State<AppState>,
    Query(params): Query<GetOnchainPublishersParams>,
) -> Result<Json<GetOnchainPublishersResponse>, EntryError> {
    let publishers = get_publishers(&state.onchain_pool, params.network)
        .await
        .map_err(EntryError::from)?;

    let publishers_with_components = get_publishers_with_components(
        &state.onchain_pool,
        params.network,
        params.data_type,
        publishers,
        state.caches.onchain_publishers_updates(),
        state.caches.onchain_decimals(),
        &state.rpc_clients,
    )
    .await
    .map_err(EntryError::from)?;

    Ok(Json(GetOnchainPublishersResponse(
        publishers_with_components,
    )))
}
