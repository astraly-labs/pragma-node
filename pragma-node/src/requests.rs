use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;
use utoipa::ToSchema;

pub const AGGREGATION_METHODS: [&str; 3] = [AGGREGATION_METHOD_MEDIAN, AGGREGATION_METHOD_RWM, AGGREGATION_METHOD_VWMP];
pub const AGGREGATION_METHOD_MEDIAN: &str = "median";
pub const AGGREGATION_METHOD_RWM: &str = "RWM";
pub const AGGREGATION_METHOD_VWMP: &str = "VWMP";

#[derive(Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct BaseEntry {
    pub timestamp: u64,
    pub source: String,
    pub publisher: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct Entry {
    pub base: BaseEntry,
    pub pair_id: String,
    pub price: u128,
    pub volume: u128,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateSpotRequest {
    pub signature: Vec<FieldElement>,
    pub pair_id: String,
    pub publisher_id: i32,
    pub data_range: SpotTimeStamp,
    pub price: u128,
    pub volume: u128,
    pub entries: Vec<Entry>
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct SpotTimeStamp {
    pub start_timestamp: u64,
    pub end_timestamp: u64,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct CreateSpotResponse {
    pub pair_id: String,
    pub price: u128,
    pub data_range: SpotTimeStamp,
    pub volume: u128,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePerpRequest {
    pub signature: Vec<FieldElement>,
    pub pair_id: String,
    pub publisher_id: i32,
    pub data_range: SpotTimeStamp,
    pub price: u128,
    pub open_interest: u128,
    pub funding_rate: u128,
    pub volume: u128,
    pub entries: Vec<Entry>
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct CreatePerpResponse {
    pub pair_id: String,
    pub price: u128,
    pub basis: u128,
    pub open_interest: u128,
    pub funding_rate: u128,
    pub data_range: SpotTimeStamp,
    pub volume: u128,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateEntryRequest {
    pub signature: Vec<FieldElement>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateEntryResponse {
    pub number_entries_created: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetEntryResponse {
    pub num_sources_aggregated: usize,
    pub pair_id: String,
    pub price: String,
    pub timestamp: u64,
    pub decimals: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetVolatilityResponse {
    pub pair_id: String,
    pub volatility: f64,
    pub decimals: u32,
}

#[derive(Deserialize)]
pub struct GetQueryParams {
    aggregation_method: Option<String>,
    pub(crate) timestamp: Option<u64>,
    pub(crate) interval: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetSpotResponse {
    pub num_sources_aggregated: usize,
    pub pair_id: String,
    pub price: String,
    pub volume: u128,
    pub data: Vec<GetSpotData>,
    pub data_range: SpotTimeStamp,
    pub decimals: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetSpotData {
    pub timestamp: u64,
    pub price: String,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetPerpResponse {
    pub num_sources_aggregated: usize,
    pub pair_id: String,
    pub price: String,
    pub timestamp: u64,
    pub volume: u128,
    pub basis: u128,
    pub open_interest: u128,
    pub funding_rate: u128,
    pub decimals: u32,
}

impl Default for GetQueryParams {
    fn default() -> Self {
        GetQueryParams {
            aggregation_method: Some(AGGREGATION_METHOD_MEDIAN.to_string()),
            timestamp: Some(chrono::Utc::now().timestamp_millis() as u64),
            interval: Some("1s".to_string()),
        }
    }
}
