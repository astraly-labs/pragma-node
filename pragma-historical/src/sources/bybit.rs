use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const LIMIT: i32 = 200; // Maximum limit allowed by Bybit

static FUNDING_INTERVALS: OnceLock<HashMap<String, u64>> = OnceLock::new();

pub struct Bybit;

#[derive(Debug, Deserialize)]
pub struct BybitFundingRateEntry {
    pub symbol: String,
    #[serde(rename = "fundingRate")]
    pub funding_rate: String,
    #[serde(rename = "fundingRateTimestamp")]
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct BybitResponse {
    #[serde(rename = "retCode")]
    pub ret_code: i32,
    #[serde(rename = "retMsg")]
    pub ret_msg: String,
    pub result: BybitResult,
}

#[derive(Debug, Deserialize)]
pub struct BybitResult {
    pub list: Vec<BybitFundingRateEntry>,
}

#[derive(Debug, Deserialize)]
pub struct InstrumentsInfoResponse {
    #[serde(rename = "retCode")]
    pub ret_code: i32,
    #[serde(rename = "retMsg")]
    pub ret_msg: String,
    pub result: InstrumentsInfoResult,
}

#[derive(Debug, Deserialize)]
pub struct InstrumentsInfoResult {
    pub list: Vec<InstrumentInfo>,
}

#[derive(Debug, Deserialize)]
pub struct InstrumentInfo {
    pub symbol: String,
    #[serde(rename = "baseCoin")]
    pub base_coin: String,
    #[serde(rename = "fundingInterval")]
    pub funding_interval: u64,
}

impl Bybit {
    pub fn get_funding_interval(base_coin: &str) -> Result<u64> {
        FUNDING_INTERVALS
            .get()
            .ok_or_else(|| anyhow!("Funding intervals not initialized"))?
            .get(base_coin)
            .copied()
            .ok_or_else(|| anyhow!("No funding interval found for {}", base_coin))
    }

    pub async fn fetch_historical_fundings(
        market: &str,
        start: i64,
        end: i64,
        client: &Client,
    ) -> Result<Vec<BybitFundingRateEntry>> {
        // Initialize funding intervals if not already done
        if FUNDING_INTERVALS.get().is_none() {
            let intervals = Self::fetch_funding_intervals(client).await?;
            FUNDING_INTERVALS
                .set(intervals)
                .map_err(|_| anyhow!("Failed to set funding intervals"))?;
        }

        // First try with PERP suffix
        let perp_market = format!("{market}PERP");
        let perp_result =
            Self::fetch_historical_fundings_for_symbol(&perp_market, start, end, client).await;

        if perp_result.is_ok() {
            return perp_result;
        }

        // If PERP fails, try with USDT suffix
        let usdt_market = format!("{market}USDT");
        Self::fetch_historical_fundings_for_symbol(&usdt_market, start, end, client).await
    }

    async fn fetch_funding_intervals(client: &Client) -> Result<HashMap<String, u64>> {
        let url = "https://api.bybit.com/v5/market/instruments-info?category=linear";
        let response = client.get(url).send().await?.error_for_status()?;
        let info: InstrumentsInfoResponse = response.json().await?;

        if info.ret_code != 0 {
            return Err(anyhow!("Bybit API error: {}", info.ret_msg));
        }

        let mut intervals = HashMap::new();
        for instrument in info.result.list {
            let ticker = instrument.base_coin;
            let symbol = instrument.symbol;
            let usd_format = format!("{ticker}USDT");
            if symbol == usd_format {
                println!(
                    "Found funding interval for {}: {} minutes",
                    ticker, instrument.funding_interval
                );
                intervals.insert(ticker, instrument.funding_interval);
            }
        }

        Ok(intervals)
    }

    async fn fetch_historical_fundings_for_symbol(
        symbol: &str,
        start: i64,
        end: i64,
        client: &Client,
    ) -> Result<Vec<BybitFundingRateEntry>> {
        let mut all_results = Vec::new();
        let mut current_start = start;

        while current_start < end {
            let response = client
                .get("https://api.bybit.com/v5/market/funding/history")
                .query(&[
                    ("category", "linear"),
                    ("symbol", symbol),
                    ("startTime", &current_start.to_string()),
                    ("endTime", &end.to_string()),
                    ("limit", &LIMIT.to_string()),
                ])
                .send()
                .await?
                .error_for_status()?
                .json::<BybitResponse>()
                .await?;

            if response.ret_code != 0 {
                return Err(anyhow!("Bybit API error: {}", response.ret_msg));
            }

            let entries = response.result.list;
            if entries.is_empty() {
                break;
            }

            // Update current_start to the timestamp of the last entry + 1ms
            if let Some(last_entry) = entries.last() {
                let last_timestamp = last_entry.timestamp.parse::<i64>()?;
                current_start = last_timestamp + 1;
            } else {
                break;
            }

            all_results.extend(entries);

            // If we got less than the limit, we've reached the end
            if all_results.len() % LIMIT as usize != 0 {
                break;
            }

            // Add a small delay to avoid rate limiting
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        Ok(all_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_historical_fundings() {
        let client = Client::new();
        let market = "BTC";
        let start = 1_746_057_600_000; // Same timestamp as other tests
        let end = start + 86_400_000; // One day later

        let result = Bybit::fetch_historical_fundings(market, start, end, &client)
            .await
            .expect("Failed to fetch historical fundings");

        assert!(!result.is_empty(), "No funding data returned");
        for entry in &result {
            assert!(
                entry.funding_rate.parse::<f64>().is_ok(),
                "Invalid funding rate"
            );
            assert!(
                entry.timestamp.parse::<i64>().is_ok(),
                "Invalid timestamp format: {}",
                entry.timestamp
            );
            // Verify timestamps are within range
            let ts = entry.timestamp.parse::<i64>().unwrap();
            assert!(
                ts >= start && ts <= end,
                "Timestamp {ts} outside range [{start}, {end}]"
            );
        }
    }
}
