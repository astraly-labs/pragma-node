pub mod sources;

use std::time::Duration;

use chrono::{DateTime, Utc};
use clap::Parser;
use deadpool_diesel::postgres::{Manager, Pool};
use reqwest::Client;

use pragma_entities::{
    error::InfraError,
    models::funding_rate::{FundingRate, NewFundingRate},
};
use sources::{hyperliquid::Hyperliquid, paradex::Paradex};

#[derive(Parser, Debug)]
#[command(name = "pragma-historical")]
struct Cli {
    /// Database URL for offchain database
    #[arg(long)]
    db_url: String,

    /// Type of data to ingest (e.g. funding_rate)
    // TODO: Clean enum
    #[arg(long)]
    data_type: String,

    /// Source of historical data (e.g. hyperliquid, paradex)
    #[arg(long)]
    source: String,

    /// Range in unix milliseconds as start,end
    #[arg(long)]
    range: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let parts: Vec<&str> = cli.range.split(',').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid range format; expected start,end but got '{}'",
            cli.range
        )
        .into());
    }
    let start: i64 = parts[0]
        .parse()
        .map_err(|e| format!("Invalid start timestamp '{}': {}", parts[0], e))?;
    let end: i64 = parts[1]
        .parse()
        .map_err(|e| format!("Invalid end timestamp '{}': {}", parts[1], e))?;

    // Build database pool
    let manager = Manager::new(
        format!("{}?application_name=pragma_historical", cli.db_url),
        deadpool_diesel::Runtime::Tokio1,
    );

    let pool = Pool::builder(manager)
        .build()
        .expect("Could not build a connection to the DB");

    if cli.data_type != "funding_rate" {
        return Err(format!(
            "Unsupported data type '{}' - only 'funding_rate' is supported",
            cli.data_type
        )
        .into());
    }

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Prepare new funding rate entries based on source
    let mut entries = Vec::new();
    match cli.source.as_str() {
        "hyperliquid" => {
            let rows =
                Hyperliquid::fetch_historical_fundings(&cli.source, start, end, &client).await?;
            if rows.is_empty() {
                println!("No historical data returned");
                return Ok(());
            }
            for r in rows {
                let ts = DateTime::<Utc>::from_timestamp_millis(r.time)
                    .expect("Invalid HL timestamp")
                    .naive_utc();
                let hourly_rate: f64 = r.funding_rate.parse()?;
                let annualized_rate = hourly_rate * 24.0 * 365.0;
                entries.push(NewFundingRate {
                    source: "hyperliquid".to_string(),
                    pair: r.coin,
                    annualized_rate,
                    timestamp: ts,
                });
            }
        }
        "paradex" => {
            let rows = Paradex::fetch_historical_fundings(&cli.source, start, end, &client).await?;
            if rows.is_empty() {
                println!("No historical data returned");
                return Ok(());
            }
            for r in rows {
                let dt = DateTime::<Utc>::from_timestamp_millis(r.created_at)
                    .expect("Invalid Paradex timestamp")
                    .naive_utc();
                let rate: f64 = r.funding_rate.parse()?;
                entries.push(NewFundingRate {
                    source: "paradex".to_string(),
                    pair: r.market,
                    annualized_rate: rate,
                    timestamp: dt,
                });
            }
        }
        other => return Err(format!("Source '{}' not implemented", other).into()),
    }

    // Insert into DB
    // TODO: Insert everything into a csv & batch insert to TS db
    let conn = pool.get().await?;
    let inserted: Vec<FundingRate> = conn
        .interact(move |conn| FundingRate::create_many(conn, entries))
        .await?
        .map_err(InfraError::from)?;
    println!("Inserted {} entries", inserted.len());

    Ok(())
}
