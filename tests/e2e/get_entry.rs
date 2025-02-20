use bigdecimal::{BigDecimal, FromPrimitive};
use rstest::rstest;

use pragma_common::types::{AggregationMode, Interval};
use serde::{Deserialize, Serialize};

use crate::{
    assert_hex_prices_within_threshold,
    common::{
        constants::EMPTY_SIGNATURE,
        setup::{setup_containers, TestHelper},
    },
};

#[rstest]
#[case::one_second(Interval::OneSecond)]
#[case::five_seconds(Interval::FiveSeconds)]
#[case::one_minute(Interval::OneMinute)]
#[case::fifteen_minutes(Interval::FifteenMinutes)]
#[case::one_hour(Interval::OneHour)]
#[case::two_hours(Interval::TwoHours)]
#[case::one_day(Interval::OneDay)]
#[case::one_week(Interval::OneWeek)]
#[serial_test::serial]
#[tokio::test]
async fn get_entry_median_ok(
    #[future] setup_containers: TestHelper,
    #[case] queried_interval: Interval,
) {
    let mut hlpr = setup_containers.await;

    // TODO: Insert way MORE entries, maybe we should have an utils to generate
    // random fake data with some constraints?
    // 1. Insert one entry
    let pair_id = "ETH/USD";
    let current_timestamp: u64 = 1739688964;
    let price: u128 = 270553000000; // 8 decimals
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

    let queried_aggregation = AggregationMode::Median;

    // 2. Refresh the timescale view
    hlpr.refresh_offchain_continuous_aggregate(
        current_timestamp,
        queried_interval,
        queried_aggregation,
    )
    .await;

    // 3. Call the endpoint
    let endpoint = get_entry_endpoint(
        "ETH",
        "USD",
        GetEntryRequestParams::new()
            .with_timestamp(current_timestamp)
            .with_interval(queried_interval)
            .with_routing(false)
            .with_aggregation(queried_aggregation),
    );
    tracing::info!("with endpoint: {endpoint}");

    let response = reqwest::get(hlpr.endpoint(&endpoint))
        .await
        .expect("Error while fetching data from pragma node")
        .json::<GetEntryResponse>()
        .await
        .expect("Could not retrieve a valid GetEntryResponse");

    hlpr.shutdown_local_pragma_node().await;

    // 4. Assert
    let expected_price_hex = format!("0x{price:x}");

    let threshold = BigDecimal::from_f64(1.0).unwrap();
    // NOTE: approx_percentile of timescaledb returns an approximative value for the median.
    // So we just check if the price we have is in ~1% bonds.
    assert_hex_prices_within_threshold!(&response.price, &expected_price_hex, threshold);
}

// Utils to call the get entry endpoint

#[derive(Default)]
pub struct GetEntryRequestParams {
    pub timestamp: Option<u64>,
    pub interval: Option<Interval>,
    pub routing: Option<bool>,
    pub aggregation: Option<AggregationMode>,
}

impl GetEntryRequestParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
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
