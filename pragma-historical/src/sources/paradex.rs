use serde::Deserialize;
use std::time::Duration;

pub struct Paradex;

#[derive(Deserialize, Debug)]
pub struct ParadexFundingRateEntry {
    pub market: String,
    #[serde(alias = "fundingRate", alias = "rate", alias = "funding_rate")]
    pub funding_rate: String,
    #[serde(alias = "created_at")]
    pub created_at: i64,
}

impl Paradex {
    pub async fn fetch_historical_fundings(
        market: &str,
        start: i64,
        end: i64,
        client: &reqwest::Client,
    ) -> Result<Vec<ParadexFundingRateEntry>, Box<dyn std::error::Error>> {
        let mut result = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let start_str = start.to_string();
            let end_str = end.to_string();
            let mut query = vec![
                ("market", market),
                ("start_at", &start_str),
                ("end_at", &end_str),
                ("page_size", "5000"),
            ];
            if let Some(ref c) = cursor {
                query.push(("cursor", c));
            }

            let payload: serde_json::Value = client
                .get("https://api.prod.paradex.trade/v1/funding/data")
                .header("Accept", "application/json")
                .query(&query)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            let rows: Vec<ParadexFundingRateEntry> = serde_json::from_value(
                payload
                    .get("results")
                    .ok_or("Missing results field")?
                    .clone(),
            )?;

            if rows.is_empty() {
                break;
            }

            result.extend(rows);
            cursor = payload
                .get("next")
                .and_then(|v| v.as_str())
                .map(String::from);
            
            println!("cursor: {:?}, created: {}", cursor, result.last().unwrap().created_at);

            if cursor.is_none() {
                break;
            }

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

        // Test with a smaller range: Jan 1, 2025, to May 15, 2025
        let start = 1704067200000; // Jan 1, 2024, 00:00:00 UTC
        let end = 1719791999999;   // Jun 30, 2024, 23:59:59 UTC

        let result = Paradex::fetch_historical_fundings("BTC-USD-PERP", start, end, &client)
            .await
            .expect("Failed to fetch funding rates");

        // Check that we get data
        assert!(!result.is_empty(), "Expected non-empty funding rate data");

        // Verify all entries are for BTC-USD-PERP and within time range
        for entry in &result {
            assert_eq!(entry.market, "BTC-USD-PERP");
            assert!(
                entry.created_at >= start && entry.created_at <= end,
                "Timestamp {} is outside range {} to {}",
                entry.created_at,
                start,
                end
            );

        }

        // Expect at least 300 entries (conservative, ~3 entries/day for ~135 days)
        assert!(
            result.len() >= 300,
            "Expected at least 300 funding rate entries, got {}",
            result.len()
        );

        // Verify first and last timestamps are close to start and end
        let first_time = result.first().unwrap().created_at;
        let last_time = result.last().unwrap().created_at;
        let eight_hours_ms = 8 * 60 * 60 * 1000; // 8 hours in milliseconds

        assert!(
            (first_time - start).abs() <= eight_hours_ms,
            "First timestamp ({}) too far from start ({}), difference: {}ms",
            first_time,
            start,
            (first_time - start).abs()
        );
        assert!(
            (end - last_time).abs() <= eight_hours_ms,
            "Last timestamp ({}) too far from end ({}), difference: {}ms",
            last_time,
            end,
            (end - last_time).abs()
        );
    }
}