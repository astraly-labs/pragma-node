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
    ) -> anyhow::Result<Vec<HyperliquidFundingRateEntry>> {
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