mod common;

use httpmock::MockServer;
use rstest::*;
use starknet::core::types::Felt;

use pragma_common::{hash::pedersen_hash, instrument, types::Network};
use pragma_consumer::{
    builder::PragmaConsumerBuilder,
    config::{ApiConfig, PragmaBaseUrl},
    consumer::PragmaConsumer,
    types::{BlockId, BlockTag, Instrument},
};

use common::mocks::{
    merkle_root_data, mock_healthcheck, mock_merkle_proof_response, mock_option_response,
    option_data,
};

#[rstest]
#[tokio::test]
async fn test_consumer() {
    let pragmapi = MockServer::start();

    let api_config = ApiConfig {
        base_url: PragmaBaseUrl::Custom(format!("http://{}", pragmapi.address())),
        api_key: "this_is_a_test".into(),
    };

    let healthcheck_mock = mock_healthcheck(&pragmapi);

    // 1. Build the consumer with an healthcheck
    let consumer: PragmaConsumer = PragmaConsumerBuilder::new()
        .on_sepolia()
        .check_api_health()
        .with_http(api_config)
        .await
        .expect("Could not build PragmaConsumer");
    healthcheck_mock.assert();

    // 2. Define some fake tests instruments
    let test_instrument: Instrument = instrument!("BTC-16AUG24-52000-P");
    let block_test = BlockId::Tag(BlockTag::Latest);
    let network = Network::Sepolia;

    // 2.5 Mock responses
    let option_mock = mock_option_response(&pragmapi, test_instrument.clone(), network, block_test);
    let merkle_proof_mock = mock_merkle_proof_response(
        &pragmapi,
        option_data(&test_instrument)["hash"]
            .as_str()
            .unwrap()
            .to_owned(),
        network,
        block_test,
    );

    // 3. Fetch the calldata & assert that the mocks got correctly called
    let calldata = consumer
        .get_merkle_feed_calldata(&test_instrument, Some(block_test))
        .await
        .expect("Could not fetch the calldata");

    option_mock.assert();
    merkle_proof_mock.assert();

    // 4. Verify the proof returned
    let expected_merkle_root = Felt::from_hex(&merkle_root_data()).unwrap();

    let mut out_merkle_root = calldata
        .option_data
        .pedersen_hash()
        .expect("Could not generate the hash of option");

    for sibling in calldata.merkle_proof.0 {
        let felt_sibling = Felt::from_hex(&sibling).unwrap();
        out_merkle_root = pedersen_hash(&out_merkle_root, &felt_sibling);
    }

    assert_eq!(out_merkle_root, expected_merkle_root);
}
