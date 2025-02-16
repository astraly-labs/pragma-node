use std::time::Duration;

use rstest::rstest;

use pragma_common::types::{AggregationMode, Interval};
use serde::{Deserialize, Serialize};

use crate::common::{
    constants::EMPTY_SIGNATURE,
    setup::{setup_containers, TestHelper},
};

#[rstest]
#[serial_test::serial]
#[tokio::test]
async fn get_entry_ok(#[future] setup_containers: TestHelper) {
    let hlpr = setup_containers.await;

    // 1. Insert one entry
    let pair_id = "ETH/USD";
    let current_timestamp = 1739688964;
    let price: u128 = 2705530000000000000000; // 18 decimals
    let publisher = "TEST_PUBLISHER";

    let sql = format!(
        r#"
        INSERT INTO entries (
            pair_id,
            publisher,
            timestamp,
            price,
            source,
            publisher_signature
        ) VALUES (
            '{pair_id}',
            '{publisher}',
            to_timestamp({current_timestamp}),
            {price},
            'BINANCE',
            '{EMPTY_SIGNATURE}'
        );
    "#
    );
    hlpr.execute_sql(&hlpr.offchain_pool, sql).await;

    // 2. Call the endpoint
    let endpoint = get_entry_endpoint(
        "ETH",
        "USD",
        GetEntryRequestParams::new()
            .with_timestamp(current_timestamp)
            .with_interval(Interval::FiveSeconds)
            .with_routing(false)
            .with_aggregation(AggregationMode::Median),
    );
    tracing::info!("with endpoint: {endpoint}");

    // Sleep some time so the 5s interval gets filled by timescale
    // TODO: Can we refresh it ourselves?
    tokio::time::sleep(Duration::from_secs(10)).await;

    let response = reqwest::get(hlpr.endpoint(&endpoint))
        .await
        .unwrap()
        .json::<GetEntryResponse>()
        .await
        .unwrap();

    // 3. Assert
    let expected_response = GetEntryResponse {
        num_sources_aggregated: 1,
        pair_id: "ETH/USD".into(),
        price: format!("0x{price:x}"),
        timestamp: 1739688964 * 1000, // in ms
        decimals: 8,
    };
    assert_eq!(response, expected_response);
}

// Utils to call the get entry endpoint

#[derive(Default)]
pub struct GetEntryRequestParams {
    pub timestamp: Option<i64>,
    pub interval: Option<Interval>,
    pub routing: Option<bool>,
    pub aggregation: Option<AggregationMode>,
}

impl GetEntryRequestParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn with_interval(mut self, interval: Interval) -> Self {
        self.interval = Some(interval);
        self
    }

    pub fn with_routing(mut self, routing: bool) -> Self {
        self.routing = Some(routing);
        self
    }

    pub fn with_aggregation(mut self, aggregation: AggregationMode) -> Self {
        self.aggregation = Some(aggregation);
        self
    }
}

pub fn get_entry_endpoint(base: &str, quote: &str, params: GetEntryRequestParams) -> String {
    let mut query = Vec::new();

    if let Some(timestamp) = params.timestamp {
        query.push(format!("timestamp={}", timestamp));
    }

    if let Some(interval) = params.interval {
        let interval = serde_json::to_string(&interval).unwrap().replace('"', "");
        query.push(format!("interval={}", interval));
    }

    if let Some(routing) = params.routing {
        query.push(format!("routing={}", routing));
    }

    if let Some(aggregation) = params.aggregation {
        let aggregation = serde_json::to_string(&aggregation)
            .unwrap()
            .replace('"', "");
        query.push(format!("aggregation={}", aggregation));
    }

    let path = format!("node/v1/data/{}/{}", base, quote);

    if query.is_empty() {
        path
    } else {
        format!("{}?{}", path, query.join("&"))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetEntryResponse {
    num_sources_aggregated: usize,
    pair_id: String,
    price: String,
    timestamp: u64,
    decimals: u32,
}
