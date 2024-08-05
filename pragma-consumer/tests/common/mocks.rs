use httpmock::{prelude::*, Mock};
use pragma_common::types::Network;
use pragma_consumer::Instrument;
use serde_json::json;

pub fn mock_healthcheck(pragmapi: &MockServer) -> Mock {
    pragmapi.mock(|when, then| {
        when.method(GET).path("/node");
        then.status(200).body("Server is running!");
    })
}

pub fn mock_option_response(
    pragmapi: &MockServer,
    instrument: Instrument,
    network: Network,
    block_number: u64,
) -> Mock {
    let url = format!("node/v1/merkle_feeds/options/{}", instrument.name(),);
    pragmapi.mock(|when, then| {
        when.method(GET)
            .path_contains(url)
            .query_param("network", network.to_string())
            .query_param("block_number", block_number.to_string());
        then.status(200)
            .header("content-type", "text/json")
            .json_body(option_data(&instrument));
    })
}

pub fn mock_merkle_proof_response(
    pragmapi: &MockServer,
    option_hash: String,
    network: Network,
    block_number: u64,
) -> Mock {
    let url = format!("node/v1/merkle_feeds/proof/{}", &option_hash);
    pragmapi.mock(|when, then| {
        when.method(GET)
            .path_contains(url)
            .query_param("network", network.to_string())
            .query_param("block_number", block_number.to_string());
        then.status(200)
            .header("content-type", "text/json")
            .json_body(merkle_proof_data());
    })
}

pub fn option_data(instrument: &Instrument) -> serde_json::Value {
    json!({
        "instrument_name": instrument.name(),
        "base_currency": &instrument.base_currency.to_string(),
        "current_timestamp": 1722805873,
        "mark_price": "45431835920",
        "hash": "0x7866fd2ec3bc6bd1a2efb6e1f02337d62064a86e8d5755bdc568d92a06f320a"
    })
}

pub fn merkle_proof_data() -> serde_json::Value {
    json!([
        "0x78626d4f8f1e24c24a41d90457688b436463d7595c4dd483671b1d5297518d2",
        "0x14eb21a8e98fbd61f20d0bbdba2b32cb2bcb61082dfcf5229370aca5b2dbd2",
        "0x73a5b6ab2f3ed2647ed316e5d4acac4db4b5f8da8f6e4707e633ebe02006043",
        "0x1c156b5dedc44a27e73968ebe3d464538d7bb0332f1c8191b2eb4a5afca8c7a",
        "0x39b52ee5f605f57cc893d398b09cb558c87ec9c956e11cd066df82e1006b33b",
        "0x698ea138d770764c65cb171627c57ebc1efb7c495b2c7098872cb485fd2e0bc",
        "0x313f2d7dc97dabc9a7fea0b42a5357787cabe78cdcca0d8274eabe170aaa79d",
        "0x6b35594ee638d1baa9932b306753fbd43a300435af0d51abd3dd7bd06159e80",
        "0x6e9f8a80ebebac7ba997448a1c50cd093e1b9c858cac81537446bafa4aa9431",
        "0x3082dc1a8f44267c1b9bea29a3df4bd421e9c33ee1594bf297a94dfd34c7ae4",
        "0x16356d27fc23e31a3570926c593bb37430201f51282f2628780264d3a399867"
    ])
}

pub fn merkle_root_data() -> String {
    "0x31d84dd2db2edb4b74a651b0f86351612efdedc51b51a178d5967a3cdfd319f".into()
}
