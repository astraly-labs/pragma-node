use std::time::Duration;
use serde::Deserialize;

pub struct Hyperliquid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidFundingRateEntry {
    pub coin: String,
    pub funding_rate: String,
    pub time: i64,
}

impl Hyperliquid {
    pub async fn fetch_historical_fundings(
        coin: &str,
        start: i64,
        end: i64,
        client: &reqwest::Client,
    ) -> Result<Vec<HyperliquidFundingRateEntry>, Box<dyn std::error::Error>> {
        let mut result = Vec::new();
        let mut current = start;

        while current < end {
            let payload = serde_json::json!({
                "type": "fundingHistory",
                "coin": coin,
                "startTime": current,
                "endTime": end
            });

            let resp = client
                .post("https://api.hyperliquid.xyz/info")
                .json(&payload)
                .send()
                .await?
                .error_for_status()?
                .json::<Vec<HyperliquidFundingRateEntry>>()
                .await?;

            if resp.is_empty() {
                break;
            }

            let last_time = resp.last().unwrap().time;
            result.extend(resp);

            if last_time >= end {
                break;
            }

            current = last_time + 1;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_fetch_historical_fundings() {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        // Test range: May 12, 2023, to Dec 31, 2024
        let start = 1683849600000; // May 12, 2023, 00:00:00 UTC
        let end = 1735689599999;   // Dec 31, 2024, 23:59:59 UTC

        let result = Hyperliquid::fetch_historical_fundings("ETH", start, end, &client)
            .await
            .expect("Failed to fetch funding rates");

        // Verify non-empty response
        assert!(!result.is_empty(), "Expected non-empty funding rate data");

        // Verify all entries are for ETH and within time range
        for entry in &result {
            assert_eq!(entry.coin, "ETH");
            assert!(entry.time >= start && entry.time <= end);
        }

        // Expect at least 1800 entries (~3 entries/day for ~600 days)
        assert!(result.len() >= 1800, "Expected at least 1800 funding rate entries");

        // Verify first and last timestamps are within 8 hours
        let first_time = result.first().unwrap().time;
        let last_time = result.last().unwrap().time;
        let eight_hours_ms = 8 * 60 * 60 * 1000; // 8 hours in milliseconds

        assert!(
            (first_time - start).abs() <= eight_hours_ms,
            "First timestamp ({}) too far from start ({})",
            first_time,
            start
        );
        assert!(
            (end - last_time).abs() <= eight_hours_ms,
            "Last timestamp ({}) too far from end ({})",
            last_time,
            end
        );
    }
}