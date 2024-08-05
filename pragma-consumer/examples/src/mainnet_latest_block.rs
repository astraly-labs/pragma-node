use pragma_consumer::builder::PragmaConsumerBuilder;
use pragma_consumer::config::{ApiConfig, PragmaBaseUrl};
use pragma_consumer::macros::instrument;
use pragma_consumer::types::{BlockId, BlockTag, Instrument};

#[tokio::main]
async fn main() -> Result<(), ()> {
    let api_config = ApiConfig {
        base_url: PragmaBaseUrl::Dev,
        api_key: "".into(),
    };

    let consumer = PragmaConsumerBuilder::new()
        .on_mainnet() // Sepolia by default
        .with_http(api_config)
        .await
        .unwrap();

    let instrument = instrument!("BTC-16AUG24-52000-P");

    let calldata = consumer
        .get_merkle_feed_calldata(&instrument, None)
        .await
        .unwrap();

    let _ = dbg!(calldata);
    // Use the calldata with the pragma-oracle contract...
    Ok(())
}
