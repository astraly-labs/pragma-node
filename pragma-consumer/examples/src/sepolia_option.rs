use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::NaiveDate;
use color_eyre::Result;

use pragma_consumer::{Instrument, OptionCurrency, OptionType};
use pragma_consumer::builder::PragmaConsumerBuilder;
use pragma_consumer::config::ApiConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let api_config = ApiConfig {
        base_url: PragmaBaseUrl::Custom("http://localhost:3000".into()),
        api_key: "".into(),
    };

    let consumer = PragmaConsumerBuilder::new()
        .on_sepolia() // Sepolia by default
        .with_http(api_config)
        .await?;

    let instrument = Instrument {
        base_currency: OptionCurrency::BTC,
        expiration_date: NaiveDate::from_ymd_opt(2024, 8, 16).unwrap(),
        strike_price: BigDecimal::from(52000).unwrap(),
        option_type: OptionType::Put
    };
    // Or
    // let instrument = Instrument::from_name("BTC-16AUG24-52000-P")

    let current_block = 85626;
    let calldata = consumer
        .get_merkle_feed_calldata(&instrument, current_block)
        .await?;

    let _ = dbg!(calldata);
    // Use the calldata with the pragma-oracle contract...
    Ok(())
}
