use serde::Deserialize;

pub struct Paradex;

#[derive(Deserialize)]
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
        let params = vec![
            ("market", market.to_string()),
            ("start_at", start.to_string()),
            ("end_at", end.to_string()),
            ("page_size", "5000".to_string()),
        ];
        let mut cursor: Option<String> = None;
        loop {
            let mut req = client
                .get("https://api.prod.paradex.trade/v1/funding/data")
                .header("Accept", "application/json");
            for (k, v) in &params {
                req = req.query(&[(*k, v)]);
            }
            if let Some(ref c) = cursor {
                req = req.query(&[("cursor", c)]);
            }
            let payload: serde_json::Value = req.send().await?.json().await?;
            let mut rows: Vec<ParadexFundingRateEntry> =
                serde_json::from_value(payload["results"].clone())?;
            if rows.is_empty() {
                break;
            }
            cursor = payload["next"].as_str().map(|s| s.to_string());
            result.append(&mut rows);
            if cursor.is_none() {
                break;
            }
        }
        Ok(result)
    }
}
