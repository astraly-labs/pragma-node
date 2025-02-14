use pragma_consumer::builder::PragmaConsumerBuilder;
use pragma_consumer::config::{ApiConfig, PragmaBaseUrl};
use pragma_consumer::macros::instrument;
use pragma_consumer::types::Instrument;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let api_config = ApiConfig {
        base_url: PragmaBaseUrl::Custom("http://localhost:3000".into()),
        api_key: String::new(),
    };

    let consumer = PragmaConsumerBuilder::new()
        .on_sepolia() // Sepolia by default
        .with_http(api_config)
        .await
        .unwrap();

    let instrument = instrument!("BTC-16AUG24-52000-P");

    let result = consumer
        .get_merkle_feed_calldata(&instrument, None) // Pending block by default
        .await
        .unwrap();

    let _ = dbg!(&result);
    // Use the calldata with the pragma-oracle contract...
    let _ = dbg!(&result.as_hex_calldata());
    Ok(())
}
