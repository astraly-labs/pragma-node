use color_eyre::Result;

use pragma_consumer::builder::PragmaConsumerBuilder;
use pragma_consumer::config::ApiConfig;
use pragma_consumer::instrument;
use pragma_consumer::Instrument;

#[tokio::main]
async fn main() -> Result<()> {
    let api_config = ApiConfig {
        base_url: "http://localhost:3000".into(),
        api_key: "".into(),
    };

    let consumer = PragmaConsumerBuilder::new()
        .on_sepolia()
        .with_api(api_config)
        .await?;

    let current_block = 85626;
    let instrument = instrument!("BTC-16AUG24-52000-P");

    let calldata = consumer
        .get_merkle_feed_calldata(&instrument, current_block)
        .await?;

    let _ = dbg!(calldata);
    // Use the calldata with the pragma-oracle contract...
    Ok(())
}
