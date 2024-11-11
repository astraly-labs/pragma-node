use pragma_consumer::builder::PragmaConsumerBuilder;
use pragma_consumer::config::{ApiConfig, PragmaBaseUrl};
use pragma_consumer::macros::instrument;
use pragma_consumer::types::Instrument;
use reqwest::Url;
use starknet::accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
use starknet::core::types::{Call, Felt};
use starknet::core::utils::get_selector_from_name;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use starknet::signers::{LocalWallet, SigningKey};

#[tokio::main]
async fn main() -> Result<(), ()> {
    let api_config = ApiConfig {
        base_url: PragmaBaseUrl::Dev,
        api_key: "".into(),
    };

    let consumer = PragmaConsumerBuilder::new()
        .with_http(api_config)
        .await
        .unwrap();

    let instrument = instrument!("BTC-30AUG24-52000-C");

    let result = consumer
        .get_merkle_feed_calldata(&instrument, None)
        .await
        .unwrap();

    let _ = dbg!(&result);
    // Use the calldata with the pragma-oracle contract...
    let _ = dbg!(&result.as_hex_calldata());

    // Use the calldata with the pragma-oracle contract...
    let calldata = result.as_calldata().unwrap();

    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse("https://starknet-sepolia.public.blastapi.io/rpc/v0_7").unwrap(),
    ));

    let signer = LocalWallet::from(SigningKey::from_secret_scalar(
        Felt::from_hex("<YOUR_PRIVATE_KEY_HERE>").unwrap(),
    ));
    let address = Felt::from_hex("<YOUR_ACCOUNT_ADDRESS_HERE>").unwrap();
    let summary_stats_address =
        Felt::from_hex("0x0379afb83d2f8e38ab08252750233665a812a24278aacdde52475618edbf879c")
            .unwrap();

    let mut account = SingleOwnerAccount::new(
        provider,
        signer,
        address,
        Felt::from_hex("0x534e5f5345504f4c4941").unwrap(), // SN_SEPOLIA
        ExecutionEncoding::New,
    );
    account.set_block_id(starknet::core::types::BlockId::Tag(
        starknet::core::types::BlockTag::Pending,
    ));

    let result = account
        .execute_v1(vec![Call {
            to: summary_stats_address,
            selector: get_selector_from_name("update_options_data").unwrap(),
            calldata,
        }])
        .send()
        .await
        .unwrap();

    println!("Transaction hash: {:#064x}", result.transaction_hash);

    Ok(())
}
