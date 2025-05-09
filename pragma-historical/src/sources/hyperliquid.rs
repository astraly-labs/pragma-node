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
            let last = resp.last().unwrap().time;
            result.extend(resp);
            if result.len() < 500 {
                break;
            }
            current = last + 1;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Ok(result)
    }
}
