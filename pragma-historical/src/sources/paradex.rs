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
    ) -> anyhow::Result<Vec<ParadexFundingRateEntry>> {
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
                    .ok_or(anyhow::anyhow!("Missing results field"))?
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