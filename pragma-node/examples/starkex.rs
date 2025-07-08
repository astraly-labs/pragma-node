use chrono::Utc;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures_util::{SinkExt as _, StreamExt as _};
use ratatui::{
    Terminal,
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use serde::{Deserialize, Serialize};
use starknet::core::utils::parse_cairo_short_string;
use std::env;
use std::{io, sync::mpsc, thread, time::Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

const TEST_PAIRS: &[&str] = &[
    // "EUR/USD",
    // "ETH/USD",
    "SOL/USD",
    // "AVAX/USD",
    // "MATIC/USD",
    // "ARB/USD",
];

const TEST_MARK_PAIRS: &[&str] = &["BTC/USD"];

#[derive(Debug)]
enum WebSocketMessage {
    Ack(SubscriptionAck),
    Update(SubscribeToEntryResponse),
}

#[derive(Serialize, Deserialize, Debug)]
struct SubscribeMessage {
    msg_type: String,
    pairs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(unused)]
struct SignedPublisherPrice {
    oracle_asset_id: String,
    oracle_price: String,
    signing_key: String,
    timestamp: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(unused)]
struct AssetOraclePrice {
    global_asset_id: String,
    median_price: String,
    signature: String,
    signed_prices: Vec<SignedPublisherPrice>,
}

#[derive(Debug, Clone, Deserialize)]
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
            "dev" => "wss://ws.devnet.pragma.build/node/v1/data/subscribe",
            "local" => "ws://0.0.0.0:3000/node/v1/data/subscribe",
            _ => panic!("Invalid environment: {env_type}. Use 'prod', 'dev', or 'local'",),
        }
        .to_string();

        Self { ws_url }
    }
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
struct SubscriptionAck {
    msg_type: String,
    pairs: Vec<String>,
}

struct App {
    subscription_pairs: Vec<String>,
    latest_update: Option<SubscribeToEntryResponse>,
    should_quit: bool,
    current_time: i64,
}

impl App {
    fn new() -> Self {
        Self {
            subscription_pairs: Vec::new(),
            latest_update: None,
            should_quit: false,
            current_time: Utc::now().timestamp(),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create channels for WebSocket messages
    let (tx, rx) = mpsc::channel();
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
    let mut app = App::new();

    // Spawn WebSocket thread
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let env = Environment::new();

            // Convert spot pairs
            let index_pairs: Vec<String> = TEST_PAIRS.iter().map(|&p| p.to_string()).collect();

            // Convert mark pairs and add :MARK suffix
            let mark_pairs: Vec<String> = TEST_MARK_PAIRS
                .iter()
                .map(|&p| format!("{p}:MARK"))
                .collect();

            // Combine both sets of pairs
            let mut all_pairs = index_pairs.clone();
            all_pairs.extend(mark_pairs.clone());

            let url = Url::parse(&env.ws_url).unwrap();
            let (mut socket, _) = connect_async(url).await.unwrap();

            let subscribe_msg = SubscribeMessage {
                msg_type: "subscribe".to_string(),
                pairs: all_pairs.clone(),
            };

            let msg_str = serde_json::to_string(&subscribe_msg).unwrap();
            socket.send(Message::text(msg_str)).await.unwrap();

            loop {
                tokio::select! {
                    Some(message) = socket.next() => {
                        match message {
                            Ok(msg) => {
                                if let Message::Text(text) = msg {
                                    if let Ok(ack) = serde_json::from_str::<SubscriptionAck>(&text) {
                                        if tx.send(Ok(WebSocketMessage::Ack(ack))).is_err() {
                                            break;
                                        }
                                    } else if let Ok(response) = serde_json::from_str::<SubscribeToEntryResponse>(&text) {
                                        if tx.send(Ok(WebSocketMessage::Update(response))).is_err() {
                                            break;
                                        }
                                    } else if tx.send(Err("Failed to parse message".to_string())).is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("WebSocket error: {e}");
                                break;
                            }
                        }
                    }
                    Ok(()) = &mut shutdown_rx => {
                        // Clean shutdown
                        let _ = socket.close(None).await;
                        break;
                    }
                }
            }
        });
    });

    // Main loop
    loop {
        app.current_time = Utc::now().timestamp();
        terminal.draw(|f| ui(f, &app))?;

        // Handle events
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    app.should_quit = true;
                }
            }
        }

        // Check for WebSocket messages
        if let Ok(msg) = rx.try_recv() {
            match msg {
                Ok(WebSocketMessage::Ack(ack)) => {
                    app.subscription_pairs = ack.pairs;
                }
                Ok(WebSocketMessage::Update(response)) => {
                    app.latest_update = Some(response);
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }

        if app.should_quit {
            let _ = shutdown_tx.send(()); // Signal WebSocket thread to shutdown
            thread::sleep(Duration::from_millis(100)); // Brief pause to allow cleanup
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/// Converts a hexadecimal asset ID to a human-readable string format.
/// If the input starts with "0x", attempts to parse it as a Cairo short string.
/// Otherwise, returns the original string.
///
/// # Arguments
/// * `hex_id` - The hexadecimal asset ID to parse
///
/// # Returns
/// A String containing either the parsed asset name or the original hex ID
fn parse_hex_asset_id(hex_id: &str) -> String {
    if !hex_id.starts_with("0x") {
        return hex_id.to_string();
    }

    // Remove "0x" prefix and any trailing zeros
    let hex_str = hex_id[2..].trim_end_matches('0');

    // Convert hex to felt and then to string
    if let Ok(felt) = u128::from_str_radix(hex_str, 16) {
        if let Ok(s) = parse_cairo_short_string(&felt.into()) {
            // The format is always ASSET-USD-8, so we can safely remove the -8 suffix
            if s.ends_with("-8") {
                return s[..s.len() - 2].to_string();
            }
            return s;
        }
    }
    hex_id.to_string()
}

/// Extracts and formats all received pairs from oracle prices.
/// Converts from the `StarkEx` encoded format back to human-readable pairs.
///
/// # Arguments
/// * `oracle_prices` - Slice of `AssetOraclePrice` containing the received price updates
///
/// # Returns
/// A Vec<String> containing all formatted asset pairs (e.g., "BTC-USD")
fn get_received_pairs(oracle_prices: &[AssetOraclePrice]) -> Vec<String> {
    oracle_prices
        .iter()
        .map(|p| parse_hex_asset_id(&p.global_asset_id))
        .collect()
}

/// Identifies which subscribed pairs are missing from the received pairs.
/// Handles the format difference between subscribed pairs and received pairs.
///
/// # Arguments
/// * `subscribed` - Slice of subscribed pair strings (format: "BTC/USD")
/// * `received` - Slice of received pair strings (format: "BTC-USD")
///
/// # Returns
/// A Vec<String> containing all subscribed pairs that weren't received
fn get_missing_pairs(subscribed: &[String], received: &[String]) -> Vec<String> {
    subscribed
        .iter()
        .filter(|p| {
            let normalized_sub = p.replace('/', "-");
            !received.iter().any(|r| {
                let normalized_rec = r.replace('/', "-");
                normalized_sub == normalized_rec
            })
        })
        .cloned()
        .collect()
}

/// Formats the missing pairs into a user-friendly status message.
///
/// # Arguments
/// * `missing_pairs` - Slice of strings containing the missing pairs
///
/// # Returns
/// A formatted string either confirming all pairs were received or listing missing pairs
fn format_missing_pairs_text(missing_pairs: &[String]) -> String {
    if missing_pairs.is_empty() {
        "✅ All pairs received".to_string()
    } else {
        format!("⚠️  Missing: {}", missing_pairs.join(", "))
    }
}

fn ui(f: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Subscription status
            Constraint::Length(3), // Missing pairs
            Constraint::Min(10),   // Price updates
        ])
        .split(f.size());

    // Add latency display with milliseconds
    if let Some(update) = &app.latest_update {
        let latency_ms = (app.current_time - update.timestamp) * 1000; // Convert to milliseconds
        let latency_text = Paragraph::new(format!("⏱ Latency: {latency_ms}ms"))
            .alignment(Alignment::Right)
            .block(Block::default().borders(Borders::ALL));

        let latency_area = Rect {
            x: chunks[0].width - 25,
            y: 0,
            width: 25,
            height: 3,
        };
        f.render_widget(latency_text, latency_area);
    }

    // Subscription header
    let subscribed_pairs = Paragraph::new(format!(
        "Subscribed Pairs: {}",
        app.subscription_pairs.join(", ")
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Subscription Status"),
    );
    f.render_widget(subscribed_pairs, chunks[0]);

    // Missing pairs section
    if let Some(update) = &app.latest_update {
        let received_pairs = get_received_pairs(&update.oracle_prices);
        let missing_pairs = get_missing_pairs(&app.subscription_pairs, &received_pairs);
        let missing_text = format_missing_pairs_text(&missing_pairs);

        let missing_widget = Paragraph::new(missing_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Missing Pairs"),
        );
        f.render_widget(missing_widget, chunks[1]);

        // Price updates list
        let mut items = vec![];
        for price in &update.oracle_prices {
            let asset_display = parse_hex_asset_id(&price.global_asset_id);

            items.push(ListItem::new(vec![
                Line::from(format!(
                    "🔸 Asset: {} ({})",
                    asset_display, price.global_asset_id
                )),
                Line::from(format!("  ├─ Median Price: {}", price.median_price)),
                Line::from(format!("  ├─ Publishers: {}", price.signed_prices.len())),
            ]));

            for (idx, pub_price) in price.signed_prices.iter().enumerate() {
                items.push(ListItem::new(vec![
                    Line::from(format!(
                        "     {}. Price: {}",
                        idx + 1,
                        pub_price.oracle_price
                    )),
                    Line::from(format!(
                        "        Key: {}...{}",
                        &pub_price.signing_key[..10],
                        &pub_price.signing_key[pub_price.signing_key.len() - 8..]
                    )),
                ]));
            }
        }

        let prices_list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Price Updates (Timestamp: {})", update.timestamp)),
        );
        f.render_widget(prices_list, chunks[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_received_pairs() {
        let oracle_prices = vec![
            AssetOraclePrice {
                global_asset_id: "0x534f4c2d5553442d38000000000000".to_string(), // SOL-USD-8
                median_price: "100".to_string(),
                signature: "sig".to_string(),
                signed_prices: vec![],
            },
            AssetOraclePrice {
                global_asset_id: "0x4254432d5553442d38000000000000".to_string(), // BTC-USD-8
                median_price: "100".to_string(),
                signature: "sig".to_string(),
                signed_prices: vec![],
            },
        ];

        let received = get_received_pairs(&oracle_prices);
        assert_eq!(received, vec!["SOL-USD", "BTC-USD"]);
    }

    #[test]
    fn test_get_missing_pairs() {
        let subscribed = vec![
            "SOL/USD".to_string(),
            "BTC/USD".to_string(),
            "ETH/USD".to_string(),
        ];
        let received = vec!["SOL-USD".to_string(), "BTC-USD".to_string()];

        let missing = get_missing_pairs(&subscribed, &received);
        assert_eq!(missing, vec!["ETH/USD"]);
    }

    #[test]
    fn test_get_missing_pairs_with_mixed_separators() {
        let subscribed = vec![
            "SOL/USD".to_string(),
            "BTC-USD".to_string(),
            "ETH/USD".to_string(),
        ];
        let received = vec!["SOL-USD".to_string(), "BTC/USD".to_string()];

        let missing = get_missing_pairs(&subscribed, &received);
        assert_eq!(missing, vec!["ETH/USD"]);
    }

    #[test]
    fn test_get_missing_pairs_all_present() {
        let subscribed = vec!["SOL/USD".to_string(), "BTC/USD".to_string()];
        let received = vec!["SOL-USD".to_string(), "BTC-USD".to_string()];

        let missing = get_missing_pairs(&subscribed, &received);
        assert!(missing.is_empty(), "Expected no missing pairs");
    }

    #[test]
    fn test_parse_hex_asset_id() {
        let test_cases = vec![
            ("0x534f4c2d5553442d38000000000000", "SOL-USD"),
            ("0x4254432d5553442d38000000000000", "BTC-USD"),
            ("0x4554482d5553442d38000000000000", "ETH-USD"),
        ];

        for (input, expected) in test_cases {
            let result = parse_hex_asset_id(input);
            assert_eq!(result, expected, "Failed to parse {}", input);
        }
    }
}
