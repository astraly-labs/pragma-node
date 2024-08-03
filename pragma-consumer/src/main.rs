pub mod builder;
pub mod config;
pub(crate) mod constants;
pub mod consumer;
pub mod types;

use color_eyre::Result;

use pragma_common::instrument;
use pragma_common::types::instrument::Instrument;

use builder::PragmaConsumerBuilder;
use config::ApiConfig;

// TODO(akhercha): Delete main function. Used for testing.
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

    let instrument = instrument!("BTC-27JUN25-80000-P");
    let calldata = consumer.get_deribit_options_calldata(&instrument).await?;

    let _ = dbg!(calldata);
    Ok(())
}
