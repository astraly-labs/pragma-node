use pragma_consumer::builder::PragmaConsumerBuilder;
use pragma_consumer::config::ApiConfig;
use pragma_consumer::macros::instrument;
use pragma_consumer::types::Instrument;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let api_config = ApiConfig {
        base_url: "http://localhost:3000".into(),
        api_key: "".into(),
    };

    let consumer = PragmaConsumerBuilder::new()
        .on_sepolia()
        .check_api_health()
        .with_http(api_config)
        .await
        .unwrap();

    let current_block = 85901;
    let instrument = instrument!("BTC-16AUG24-52000-P");

    let calldata = consumer
        .get_merkle_feed_calldata(&instrument, current_block)
        .await
        .unwrap();

    let _ = dbg!(&calldata);

    let _ = dbg!(&calldata.as_hex_calldata().unwrap());

    // Use the calldata with the pragma-oracle contract...
    Ok(())
}
