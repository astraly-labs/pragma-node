use futures_util::{SinkExt as _, StreamExt as _};
use serde::{Deserialize, Serialize};
use std::env;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

const TEST_PAIRS: &[&str] = &["BTC/USD"];

#[derive(Serialize, Deserialize, Debug)]
struct SubscribeMessage {
    msg_type: String,
    pairs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SignedPublisherPrice {
    oracle_asset_id: String,
    oracle_price: String,
    signing_key: String,
    signature: String,
    timestamp: String,
}

#[derive(Debug, Deserialize)]
struct AssetOraclePrice {
    global_asset_id: String,
    median_price: String,
    signature: String,
    signed_prices: Vec<SignedPublisherPrice>,
}

#[derive(Debug, Deserialize)]
struct SubscribeToEntryResponse {
    oracle_prices: Vec<AssetOraclePrice>,
    timestamp: i64,
}

#[derive(Debug)]
struct Environment {
    ws_url: String,
}

impl Environment {
    fn new() -> Self {
        // Default to 'dev' if not specified
        let env_type = env::var("PRAGMA_ENV").unwrap_or_else(|_| "dev".to_string());

        let ws_url = match env_type.as_str() {
            "prod" => "wss://ws.pragma.build/node/v1/data/subscribe",
            "dev" => "wss://ws.dev.pragma.build/node/v1/data/subscribe",
            "local" => "ws://0.0.0.0:3000/node/v1/data/subscribe",
            _ => panic!(
                "Invalid environment: {}. Use 'prod', 'dev', or 'local'",
                env_type
            ),
        }
        .to_string();

        Environment { ws_url }
    }
}

#[derive(Debug, Deserialize)]
struct SubscriptionAck {
    msg_type: String,
    pairs: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Environment::new();
    // Convert TEST_PAIRS into index pairs
    let index_pairs: Vec<String> = TEST_PAIRS.iter().map(|&p| p.to_string()).collect();

    let mark_pairs = index_pairs
        .iter()
        .map(|p| format!("{}:MARK", p))
        .collect::<Vec<String>>();

    // Combine both sets of pairs
    let mut all_pairs = index_pairs.clone();
    all_pairs.extend(mark_pairs.clone());

    // Connect to WebSocket
    let url = Url::parse(&env.ws_url)?;
    let (mut socket, _) = connect_async(url).await?;

    // Subscribe to all pairs
    let subscribe_msg = SubscribeMessage {
        msg_type: "subscribe".to_string(),
        pairs: all_pairs.clone(),
    };

    let msg_str = serde_json::to_string(&subscribe_msg)?;
    socket.send(Message::text(msg_str)).await?;

    println!("Subscribed to {} pairs", all_pairs.len());
    println!(
        "Expecting {} index prices and {} mark prices",
        index_pairs.len(),
        mark_pairs.len()
    );

    // Process messages
    while let Some(message) = socket.next().await {
        match message {
            Ok(msg) => {
                if let Message::Text(text) = msg {
                    // First try to parse as subscription ack
                    if let Ok(ack) = serde_json::from_str::<SubscriptionAck>(&text) {
                        println!("\n✅ Subscription confirmed for {} pairs:", ack.pairs.len());
                        for pair in &ack.pairs {
                            println!("   - {}", pair);
                        }
                        continue;
                    }

                    // If not an ack, try to parse as price update
                    match serde_json::from_str::<SubscribeToEntryResponse>(&text) {
                        Ok(response) => {
                            println!("\n{}", "═".repeat(50));
                            println!("📊 Price Update @ timestamp {}", response.timestamp);
                            println!("{}", "═".repeat(50));

                            for price in &response.oracle_prices {
                                // Convert hex asset ID to readable format if possible
                                let asset_display = if price.global_asset_id.starts_with("0x") {
                                    match price.global_asset_id.as_str() {
                                        "0x425443555344" => "BTC/USD",
                                        _ => &price.global_asset_id,
                                    }
                                } else {
                                    &price.global_asset_id
                                };

                                println!(
                                    "\n🔸 Asset: {} ({})",
                                    asset_display, price.global_asset_id
                                );
                                println!("  ├─ Median Price: {}", price.median_price);
                                println!(
                                    "  ├─ Signature: {}...{}",
                                    &price.signature[..10],
                                    &price.signature[price.signature.len() - 8..]
                                );
                                println!("  └─ Publishers ({}):", price.signed_prices.len());

                                for (idx, pub_price) in price.signed_prices.iter().enumerate() {
                                    println!(
                                        "     {}. Publisher ID: {}...{}",
                                        idx + 1,
                                        &pub_price.oracle_asset_id[..14],
                                        &pub_price.oracle_asset_id
                                            [pub_price.oracle_asset_id.len() - 8..]
                                    );
                                    println!("        ├─ Price: {}", pub_price.oracle_price);
                                    println!(
                                        "        ├─ Key: {}...{}",
                                        &pub_price.signing_key[..10],
                                        &pub_price.signing_key[pub_price.signing_key.len() - 8..]
                                    );
                                    println!("        └─ Timestamp: {}", pub_price.timestamp);
                                }
                            }

                            // Check for missing pairs
                            let received_pairs: Vec<String> = response
                                .oracle_prices
                                .iter()
                                .map(|p| {
                                    if p.global_asset_id.starts_with("0x") {
                                        match p.global_asset_id.as_str() {
                                            "0x425443555344" => "BTC/USD".to_string(),
                                            _ => p.global_asset_id.clone(),
                                        }
                                    } else {
                                        p.global_asset_id.clone()
                                    }
                                })
                                .collect();

                            let missing_pairs: Vec<_> = all_pairs
                                .iter()
                                .filter(|p| !received_pairs.contains(p))
                                .collect();

                            if !missing_pairs.is_empty() {
                                println!("\n⚠️  Missing Pairs:");
                                for pair in missing_pairs {
                                    println!("   - {}", pair);
                                }
                            }
                            println!("\n{}", "═".repeat(50));
                        }
                        Err(e) => {
                            eprintln!("Error parsing message: {}", e);
                            eprintln!("Message content: {}", text);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
