use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ParadexFundingRateEntry {
    pub created_at: i64,
    pub funding_index: String,
    pub funding_premium: String,
    pub funding_rate: String,
    pub market: String,
}

#[derive(Debug, Deserialize)]
struct ParadexResponse {
    results: Vec<ParadexFundingRateEntry>,
    next: Option<String>,
}

pub struct Paradex;

impl Paradex {
    pub async fn fetch_historical_fundings(
        market: &str,
        start: i64,
        end: i64,
        client: &Client,
    ) -> Result<Vec<ParadexFundingRateEntry>> {
        const PAGE_SIZE: i32 = 5000;
        let mut all_results = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let mut params = vec![
                ("market", market.to_string()),
                ("start_at", start.to_string()),
                ("end_at", end.to_string()),
                ("page_size", PAGE_SIZE.to_string()),
            ];

            if let Some(ref c) = cursor {
                params.push(("cursor", c.to_string()));
            }

            let response = client
                .get("https://api.testnet.paradex.trade/v1/funding/data")
                .query(&params)
                .header("Accept", "application/json")
                .send()
                .await?
                .error_for_status()?
                .json::<ParadexResponse>()
                .await?;

            if response.results.is_empty() {
                break;
            }

            all_results.extend(response.results);
            cursor = response.next;

            if cursor.is_none() {
                break;
            }
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
        let market = "BTC-USD-PERP";
        let start = 1_746_057_600_000;
        let end = start + 86_400_000; // One day later

        let result = Paradex::fetch_historical_fundings(market, start, end, &client)
            .await
            .expect("Failed to fetch historical fundings");

        assert!(!result.is_empty(), "No funding data returned");
        for entry in &result {
            assert_eq!(entry.market, market, "Market mismatch");
            assert!(
                entry.created_at >= start && entry.created_at <= end,
                "Timestamp out of range"
            );
        }
    }
}
