use pragma_consumer::builder::PragmaConsumerBuilder;
use pragma_consumer::config::ApiConfig;
use pragma_consumer::instrument;
use pragma_consumer::Instrument;

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
        .await;

    if let Err(err) = &consumer {
        println!("{:?}", err);
    }

    let consumer = consumer.unwrap();

    let current_block = 85862;
    let instrument = instrument!("BTC-16AUG24-52000-P");

    let calldata = consumer
        .get_merkle_feed_calldata(&instrument, current_block)
        .await;

    if let Err(err) = &calldata {
        println!("{:?}", err);
    }

    let _ = dbg!(calldata.unwrap());
    // Use the calldata with the pragma-oracle contract...
    Ok(())
}
