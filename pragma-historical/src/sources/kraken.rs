use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

pub struct Kraken;

#[derive(Debug, Deserialize)]
pub struct KrakenFundingRateEntry {
    #[serde(rename = "fundingRate")]
    pub funding_rate: f64,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct KrakenResponse {
    pub rates: Vec<KrakenFundingRateEntry>,
}

impl Kraken {
    pub async fn fetch_historical_fundings(
        market: &str,
        start: i64,
        end: i64,
        client: &Client,
    ) -> Result<Vec<KrakenFundingRateEntry>> {
        let url = "https://futures.kraken.com/derivatives/api/v3/historical-funding-rates";

        let response = client
            .get(url)
            .query(&[("symbol", market)])
            .send()
            .await?
            .error_for_status()?
            .json::<KrakenResponse>()
            .await?;

        // Filter entries within the requested time range
        // Kraken returns timestamps in RFC3339 format
        let filtered_rates = response
            .rates
            .into_iter()
            .filter(|r| {
                chrono::DateTime::parse_from_rfc3339(&r.timestamp).is_ok_and(|ts| {
                    let ts_millis = ts.timestamp_millis();
                    ts_millis >= start && ts_millis <= end
                })
            })
            .collect();

        Ok(filtered_rates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_historical_fundings() {
        let client = Client::new();
        let market = "PF_XBTUSD";
        let start = 1_746_057_600_000; // Same timestamp as other tests
        let end = start + 86_400_000; // One day later

        let result = Kraken::fetch_historical_fundings(market, start, end, &client)
            .await
            .expect("Failed to fetch historical fundings");

        assert!(!result.is_empty(), "No funding data returned");
        for entry in &result {
            assert!(entry.funding_rate.is_finite(), "Invalid funding rate");
            // Verify timestamp parsing
            assert!(
                chrono::DateTime::parse_from_rfc3339(&entry.timestamp).is_ok(),
                "Invalid timestamp format: {}",
                entry.timestamp
            );
        }
    }
}
