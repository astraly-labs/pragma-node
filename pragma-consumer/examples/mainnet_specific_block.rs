use pragma_consumer::builder::PragmaConsumerBuilder;
use pragma_consumer::config::{ApiConfig, PragmaBaseUrl};
use pragma_consumer::macros::instrument;
use pragma_consumer::types::{BlockId, Instrument};

#[tokio::main]
async fn main() -> Result<(), ()> {
    let api_config = ApiConfig {
        base_url: PragmaBaseUrl::Prod,
        api_key: String::new(),
    };

    let consumer = PragmaConsumerBuilder::new()
        .on_mainnet() // Sepolia by default
        .with_http(api_config)
        .await
        .unwrap();

    let current_block = BlockId::Number(85925);
    let instrument = instrument!("BTC-16AUG24-52000-P");

    let result = consumer
        .get_merkle_feed_calldata(&instrument, Some(current_block))
        .await
        .unwrap();

    let _ = dbg!(&result);
    // Use the calldata with the pragma-oracle contract...
    let _ = dbg!(&result.as_hex_calldata());
    Ok(())
}
