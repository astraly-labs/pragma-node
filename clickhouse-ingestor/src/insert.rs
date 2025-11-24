use clickhouse::Client;
use anyhow::Result;

use crate::entries::{FundingRateEntry, OpenInterestEntry, PriceEntry, TradeEntry};

/// Inserts a batch of price entries into ClickHouse
pub(crate) async fn insert_price_batch(client: &Client, entries: Vec<PriceEntry>) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut insert = client.insert::<PriceEntry>("prices").await?;

    for entry in entries {
        insert.write(&entry).await?;
    }

    insert.end().await?;
    Ok(())
}

/// Inserts a batch of funding rate entries into ClickHouse
pub(crate) async fn insert_funding_rate_batch(client: &Client, entries: Vec<FundingRateEntry>) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut insert = client.insert::<FundingRateEntry>("funding_rates").await?;

    for entry in entries {
        insert.write(&entry).await?;
    }

    insert.end().await?;
    Ok(())
}

/// Inserts a batch of open interest entries into ClickHouse
pub(crate) async fn insert_open_interest_batch(client: &Client, entries: Vec<OpenInterestEntry>) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut insert = client.insert::<OpenInterestEntry>("open_interest").await?;

    for entry in entries {
        insert.write(&entry).await?;
    }

    insert.end().await?;
    Ok(())
}

/// Inserts a batch of trade entries into ClickHouse
pub(crate) async fn insert_trade_batch(client: &Client, entries: Vec<TradeEntry>) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut insert = client.insert::<TradeEntry>("trades").await?;

    for entry in entries {
        insert.write(&entry).await?;
    }

    insert.end().await?;
    Ok(())
}

