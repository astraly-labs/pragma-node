pub mod sources;

use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use clap::Parser;
use csv::Writer;
use deadpool_diesel::postgres::{Manager, Pool};
use reqwest::Client;

use pragma_entities::models::funding_rate::NewFundingRate;
use sources::{hyperliquid::Hyperliquid, paradex::Paradex};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "pragma-historical")]
struct Cli {
    /// Database URL for offchain database
    #[arg(long)]
    db_url: String,

    /// Type of data to ingest (e.g. funding_rate)
    // TODO: Clean enum
    // #[arg(long)]
    // data_type: String,

    /// Source of historical data (e.g. hyperliquid, paradex)
    #[arg(long)]
    source: String,

    /// Range in unix milliseconds as start,end
    #[arg(long)]
    range: String,

    /// Output CSV file path
    #[arg(long, default_value = "funding_rates.csv")]
    csv_output: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let (start, end) = parse_time_range(&cli.range)?;

    // let manager = Manager::new(
    //     format!("{}?application_name=pragma_historical", cli.db_url),
    //     deadpool_diesel::Runtime::Tokio1,
    // );

    // let pool = Pool::builder(manager)
    //     .build()
    //     .expect("Could not build a connection to the DB");

    // if cli.data_type != "funding_rate" {
    //     return Err(format!(
    //         "Unsupported data type '{}' - only 'funding_rate' is supported",
    //         cli.data_type
    //     )
    //     .into());
    // }

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Define coins to fetch
    let coins = vec!["BTC/USD", "ETH/USD"];

    // Fetch and process funding rates for each coin
    let mut all_entries = Vec::new();
    for coin in coins {
        println!(
            "Fetch {coin} - from: {} to: {}",
            timestamp_from_millis(start)?,
            timestamp_from_millis(end)?
        );

        let formatted_coin = format_coin_for_exchange(&cli.source, &coin);
        let entries =
            fetch_funding_rates(&cli.source, &coin, &formatted_coin, start, end, &client).await?;
        all_entries.extend(entries);
    }

    // Write to CSV
    write_to_csv(&all_entries, &cli.csv_output)?;

    // Insert into DB
    // TODO: Insert everything into a csv & batch insert to TS db
    // let conn = pool.get().await?;
    // let inserted: Vec<FundingRate> = conn
    //     .interact(move |conn| FundingRate::create_many(conn, entries))
    //     .await?
    //     .map_err(InfraError::from)?;
    // println!("Inserted {} entries", inserted.len());

    Ok(())
}

async fn fetch_funding_rates(
    source: &str,
    original_coin: &str,
    formatted_coin: &str,
    start: i64,
    end: i64,
    client: &Client,
) -> Result<Vec<NewFundingRate>, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();
    match source {
        "hyperliquid" => {
            let rows = Hyperliquid::fetch_historical_fundings(formatted_coin, start, end, client).await?;
            for r in rows {
                let ts = timestamp_from_millis(r.time)?;
                let hourly_rate: f64 = r.funding_rate.parse()?;
                let annualized_rate = hourly_rate * 24.0 * 365.0;
                entries.push(NewFundingRate {
                    source: "hyperliquid".to_string(),
                    pair: original_coin.to_string(),
                    annualized_rate,
                    timestamp: ts,
                });
            }
        }
        "paradex" => {
            let rows = Paradex::fetch_historical_fundings(formatted_coin, start, end, client).await?;
            for r in rows {
                let ts = timestamp_from_millis(r.created_at)?;
                let rate: f64 = r.funding_rate.parse()?;
                entries.push(NewFundingRate {
                    source: "paradex".to_string(),
                    pair: original_coin.to_string(),
                    annualized_rate: rate,
                    timestamp: ts,
                });
            }
        }
        other => return Err(format!("Source '{}' not implemented", other).into()),
    }
    Ok(entries)
}

fn parse_time_range(range: &str) -> Result<(i64, i64), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = range.split(',').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid range format; expected start,end but got '{}'",
            range
        )
        .into());
    }
    let start: i64 = parts[0]
        .parse()
        .map_err(|e| format!("Invalid start timestamp '{}': {}", parts[0], e))?;
    let end: i64 = parts[1]
        .parse()
        .map_err(|e| format!("Invalid end timestamp '{}': {}", parts[1], e))?;
    Ok((start, end))
}

fn write_to_csv(
    entries: &[NewFundingRate],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = Writer::from_path(output_path)?;
    for entry in entries {
        writer.write_record(&[
            Uuid::new_v4().to_string(),
            entry.source.to_uppercase().clone(),
            entry.pair.clone(), // /USD
            entry.annualized_rate.to_string(),
            entry.timestamp.to_string(),
            Utc::now().to_string(),
        ])?;
    }
    writer.flush()?;
    println!("Wrote {} entries to {}", entries.len(), output_path);
    Ok(())
}

fn format_coin_for_exchange(source: &str, coin: &str) -> String {
    match source {
        "hyperliquid" => {
            let base = coin.split('/').next().unwrap_or(coin);
            base.to_uppercase()
        }
        "paradex" => coin.replace("/", "-").to_uppercase(),
        other => panic!("Source '{}' not implemented", other),
    }
}

fn timestamp_from_millis(millis: i64) -> Result<NaiveDateTime, String> {
    DateTime::<Utc>::from_timestamp_millis(millis)
        .ok_or_else(|| format!("Invalid timestamp: {}", millis))
        .map(|dt| dt.naive_utc())
}
