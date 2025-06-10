use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtendedFundingRateEntry {
    #[serde(rename = "m")]
    pub market: String,
    #[serde(rename = "T")]
    pub created_at: i64,
    #[serde(rename = "f")]
    pub funding_rate: String,
}

#[derive(Debug, Deserialize)]
pub struct ExtendedResponse {
    pub status: String,
    pub data: Vec<ExtendedFundingRateEntry>,
    pub pagination: Option<ExtendedPagination>,
}

#[derive(Debug, Deserialize)]
pub struct ExtendedPagination {
    pub cursor: i64,
    pub count: i64,
}

#[derive(Debug, Deserialize)]
pub struct ExtendedError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct ExtendedErrorResponse {
    pub status: String,
    pub error: ExtendedError,
}

pub struct Extended;

impl Extended {
    pub async fn fetch_historical_fundings(
        market: &str,
        start: i64,
        end: i64,
        client: &Client,
    ) -> Result<Vec<ExtendedFundingRateEntry>> {
        let mut all_results = Vec::new();
        let mut cursor: Option<i64> = None;
        const LIMIT: i64 = 1000;

        loop {
            let query_params = vec![
                ("startTime", start.to_string()),
                ("endTime", end.to_string()),
                ("limit", LIMIT.to_string()),
                ("cursor", cursor.unwrap_or(0).to_string()),
            ];

            let url = format!(
                "https://api.extended.exchange/api/v1/info/{}/funding",
                market
            );

            let response = client
                .get(&url)
                .query(&query_params)
                .header("Accept", "application/json")
                .header("User-Agent", "aa")
                .send()
                .await
                .context("Failed to send request")?;

            let status = response.status();
            if !status.is_success() {
                let error_body = response
                    .text()
                    .await
                    .context("Failed to read error response body")?;
                // Try parsing as JSON error response
                if let Ok(error) = serde_json::from_str::<ExtendedErrorResponse>(&error_body) {
                    anyhow::bail!(
                        "API error: code={}, message={}",
                        error.error.code,
                        error.error.message
                    );
                } else {
                    // Handle non-JSON (e.g., HTML) response
                    anyhow::bail!(
                        "Non-JSON error response (status {}): {}",
                        status,
                        error_body
                    );
                }
            }

            let response: ExtendedResponse = response
                .json()
                .await
                .context("Failed to parse response JSON")?;

            if response.data.is_empty() {
                break;
            }

            all_results.extend(response.data);

            cursor = response
                .pagination
                .and_then(|p| if p.count > 0 { Some(p.cursor) } else { None });

            if cursor.is_none() {
                break;
            }

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
        let market = "BTC-USD";
        let start = 1_746_057_600_000; // Same timestamp as Paradex test
        let end = start + 86_400_000; // One day later

        let result = Extended::fetch_historical_fundings(market, start, end, &client)
            .await
            .expect("Failed to fetch historical fundings");

        assert!(!result.is_empty(), "No funding data returned");
        for entry in &result {
            assert_eq!(entry.market, market, "Market mismatch");
            assert!(
                entry.created_at >= start && entry.created_at <= end,
                "Timestamp out of range"
            );
            assert!(
                entry.funding_rate.parse::<f64>().is_ok(),
                "Invalid funding rate format: {}",
                entry.funding_rate
            );
        }
    }
}
