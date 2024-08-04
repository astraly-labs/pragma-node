mod common;

use httpmock::prelude::*;
use httpmock::MockServer;
use pragma_common::instrument;
use pragma_consumer::{
    builder::PragmaConsumerBuilder, config::ApiConfig, consumer::PragmaConsumer, Instrument,
};
use rstest::*;

use common::mocks::pragmapi_mock;

#[rstest]
#[tokio::test]
async fn test_consumer(#[from(pragmapi_mock)] pragmapi: MockServer) {
    let api_config = ApiConfig {
        base_url: format!("http://{}", pragmapi.address()),
        api_key: "this_is_a_test".into(),
    };

    // 1. Create the PragmaConsumer & assert that it is healthy
    let healthcheck_mock = pragmapi.mock(|when, then| {
        when.method(GET).path("/node");
        then.status(200)
            .header("content-type", "text/html")
            .body("Server is running!");
    });
    let _consumer: PragmaConsumer = PragmaConsumerBuilder::new()
        .with_api(api_config)
        .await
        .expect("Could not build API");
    // Assert that the healthcheck mock got called
    healthcheck_mock.assert();

    // 2. Define some fake tests instruments
    let _test_instrument: Instrument = instrument!("BTC-16AUG24-52000-P");
    let _block_test = 69420;

    // 2.5 Mock responses
    // TODO

    // 3. Fetch the calldata & assert that the mocks got correctly called
    // let calldata = consumer
    //     .get_merkle_feed_calldata(&test_instrument, block_test)
    //     .await
    //     .expect("Could not fetch the calldata");
}
