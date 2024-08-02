pub mod builder;
pub mod client;
pub mod config;
pub mod types;

use builder::PragmaConsumerBuilder;
use config::ApiConfig;
use types::Instrument;

// TODO: Delete main function. Used for testing.
#[tokio::main]
async fn main() -> Result<(), ()> {
    let api_config = ApiConfig {
        base_url: "http://localhost:3000".into(),
        api_key: "hiRQqrMjNK9mFQ4TLLKc54Cs6mWCKeoq7JPIrd0g".into(),
    };

    let consumer = PragmaConsumerBuilder::new()
        .on_sepolia()
        .with_api(api_config)
        .expect("Could not build the Pragma Consumer client.");

    let instrument = instrument!("BTC-27JUN25-80000-P");

    let calldata = consumer.get_deribit_options_calldata(&instrument).await;
    let _ = dbg!(calldata);

    Ok(())
}
