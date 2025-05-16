pub mod sources;

use std::{process::Command, time::Duration};

use anyhow::{Context, anyhow};
use chrono::{DateTime, NaiveDateTime, Utc};
use clap::Parser;
use csv::Writer;
use reqwest::Client;

use pragma_entities::models::funding_rate::NewFundingRate;
use sources::{hyperliquid::Hyperliquid, paradex::Paradex};
use uuid::Uuid;

pub const ALL_PAIRS: &[&str] = &[
    "AAVE/USD",
    "APT/USD",
    "ARB/USD",
    "ATOM/USD",
    "AVAX/USD",
    "BCH/USD",
    "BNB/USD",
    "BONK/USD",
    "BTC/USD",
    "CRV/USD",
    "DOG/USD",
    "DOGE/USD",
    "DOT/USD",
    "ETC/USD",
    "ETH/USD",
    "EUR/USD",
    "FIL/USD",
    "GOAT/USD",
    "HYPE/USD",
    "INJ/USD",
    "JLP/USD",
    "JTO/USD",
    "JUP/USD",
    "LDO/USD",
    "LINK/USD",
    "LTC/USD",
    "MKR/USD",
    "MOODENG/USD",
    "MOV/USD",
    "NEAR/USD",
    "OKB/USD",
    "ONDO/USD",
    "OP/USD",
    "PENDLE/USD",
    "POL/USD",
    "POPCAT/USD",
    "S/USD",
    "SEI/USD",
    "SHIB/USD",
    "SOL/USD",
    "STRK/USD",
    "SUI/USD",
    "TIA/USD",
    "TON/USD",
    "TRUMP/USD",
    "TRX/USD",
    "USDC/USD",
    "USDT/USD",
    "WIF/USD",
    "WLD/USD",
    "XRP/USD",
];

#[derive(Parser, Debug)]
#[command(name = "pragma-historical")]
struct Cli {
    /// Source of historical data (e.g. hyperliquid, paradex)
    #[arg(long)]
    source: String,

    /// Range in unix milliseconds as start,end
    #[arg(long)]
    range: String,

    /// Output CSV file path
    #[arg(long, default_value = "funding_rates.csv")]
    csv_output: String,

    /// Database connection string (e.g., postgres://user:pass@host:port/dbname)
    #[arg(long)]
    connection: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    check_timescaledb_parallel_copy()?;

    let cli = Cli::parse();

    let (start, end) = parse_time_range(&cli.range)?;

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Define coins to fetch
    let pairs = ALL_PAIRS;

    // Fetch and process funding rates for each coin
    let mut all_entries = Vec::new();
    for pair in pairs {
        println!(
            "Fetch {pair} - from: {} to: {}",
            timestamp_from_millis(start)?,
            timestamp_from_millis(end)?
        );

        let formatted_pair = format_pair_for_exchange(&cli.source, pair);
        let entries = fetch_funding_rates(&cli.source, pair, &formatted_pair, start, end, &client)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Error fetching funding rates for {pair}: {e}");
                vec![]
            });
        all_entries.extend(entries);
    }

    // Write to CSV
    write_to_csv(&all_entries, &cli.csv_output)?;

    // Import CSV to TimescaleDB
    import_to_timescaledb(&cli.connection, &cli.csv_output)?;

    Ok(())
}

fn check_timescaledb_parallel_copy() -> anyhow::Result<()> {
    Command::new("timescaledb-parallel-copy")
        .arg("--help")
        .output()
        .map_err(|_| anyhow!(
            "timescaledb-parallel-copy not installed. Please check the instructions at https://github.com/timescale/timescaledb-parallel-copy"
        ))?;
    Ok(())
}

fn import_to_timescaledb(connection: &str, csv_path: &str) -> anyhow::Result<()> {
    let table = "funding_rates";
    let output = Command::new("timescaledb-parallel-copy")
        .arg("--connection")
        .arg(connection)
        .arg("--table")
        .arg(table)
        .arg("--file")
        .arg(csv_path)
        .output()
        .context("Failed to execute timescaledb-parallel-copy")?;

    if output.status.success() {
        println!("Imported {csv_path} to TimescaleDB");
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(anyhow!("Failed to import CSV: {error}"))
    }
}

async fn fetch_funding_rates(
    source: &str,
    orginal_pair: &str,
    formatted_pair: &str,
    start: i64,
    end: i64,
    client: &Client,
) -> anyhow::Result<Vec<NewFundingRate>> {
    let mut entries = Vec::new();
    match source {
        "hyperliquid" => {
            let rows =
                Hyperliquid::fetch_historical_fundings(formatted_pair, start, end, client).await?;
            for r in rows {
                let ts = timestamp_from_millis(r.time)?;
                let hourly_rate: f64 = r.funding_rate.parse()?;
                let annualized_rate = hourly_rate * 24.0 * 365.0;
                entries.push(NewFundingRate {
                    source: "hyperliquid".to_string(),
                    pair: orginal_pair.to_string(),
                    annualized_rate,
                    timestamp: ts,
                });
            }
        }
        "paradex" => {
            let rows =
                Paradex::fetch_historical_fundings(formatted_pair, start, end, client).await?;
            for r in rows {
                let ts = timestamp_from_millis(r.created_at)?;
                let rate: f64 = r.funding_rate.parse()?;
                entries.push(NewFundingRate {
                    source: "paradex".to_string(),
                    pair: orginal_pair.to_string(),
                    annualized_rate: rate,
                    timestamp: ts,
                });
            }
        }
        other => return Err(anyhow!("Source '{}' not implemented", other)),
    }
    Ok(entries)
}

fn parse_time_range(range: &str) -> anyhow::Result<(i64, i64)> {
    let parts: Vec<&str> = range.split(',').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid range format; expected start,end but got '{}'",
            range
        ));
    }
    let start: i64 = parts[0]
        .parse()
        .context(format!("Invalid start timestamp '{}'", parts[0]))?;
    let end: i64 = parts[1]
        .parse()
        .context(format!("Invalid end timestamp '{}'", parts[1]))?;
    Ok((start, end))
}

fn write_to_csv(entries: &[NewFundingRate], output_path: &str) -> anyhow::Result<()> {
    let mut writer = Writer::from_path(output_path)?;
    for entry in entries {
        writer.write_record(&[
            Uuid::new_v4().to_string(),
            entry.source.to_uppercase().clone(),
            entry.pair.clone(),
            entry.annualized_rate.to_string(),
            entry.timestamp.to_string(),
            Utc::now().to_string(),
        ])?;
    }
    writer.flush()?;
    println!("Wrote {} entries to {}", entries.len(), output_path);
    Ok(())
}

fn format_pair_for_exchange(source: &str, pair: &str) -> String {
    match source {
        "hyperliquid" => pair.split('/').next().unwrap_or(pair).to_uppercase(),
        "paradex" => format!("{}-PERP", pair.replace('/', "-").to_uppercase()),
        other => panic!("Source '{other}' not implemented"),
    }
}

fn timestamp_from_millis(millis: i64) -> anyhow::Result<NaiveDateTime> {
    DateTime::<Utc>::from_timestamp_millis(millis)
        .ok_or_else(|| anyhow!("Invalid timestamp: {}", millis))
        .map(|dt| dt.naive_utc())
}
