pub mod funding_rates;
pub mod get_entry;
pub mod get_ohlc;
pub mod onchain;
pub mod stream;
pub mod websocket;
pub mod open_interest;

pub use get_entry::get_entry;
pub use get_ohlc::get_ohlc;

use pragma_entities::UnixTimestamp;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use pragma_common::{AggregationMode, InstrumentType, Interval};

#[derive(Default, Debug, Deserialize, ToSchema, Clone, Copy)]
pub enum EntryType {
    #[serde(rename = "spot")]
    #[default]
    Spot,
    #[serde(rename = "perp")]
    Perp,
    #[serde(rename = "future")]
    Future,
}

impl From<EntryType> for InstrumentType {
    fn from(value: EntryType) -> Self {
        match value {
            EntryType::Spot => Self::Spot,
            EntryType::Future | EntryType::Perp => Self::Perp,
        }
    }
}

/// Parameters for retrieving price entries
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct GetEntryParams {
    /// The unix timestamp in seconds to retrieve historical price data.
    /// This endpoint will return the first update whose timestamp is <= the provided value.
    ///
    /// If not provided, returns the latest available price.
    ///
    /// # Examples
    /// - `1_647_820_800`: Returns price data from March 21, 2022 00:00:00 UTC
    /// - `null`: Returns the most recent price update
    ///
    /// NOTE: This only works for `median` aggregation
    #[schema(value_type = i64, example = 1_647_820_800)]
    pub timestamp: Option<UnixTimestamp>,

    /// Time interval for aggregated price data. Different intervals affect how price data is
    /// aggregated and can be used to get OHLC (Open/High/Low/Close) data at various timeframes.
    ///
    /// # Available intervals
    /// - `100ms`: 100 milliseconds - High frequency trading
    /// - `1s`: 1 second - Real-time trading
    /// - `5s`: 5 seconds - Short-term price movements
    /// - `1min`: 1 minute - Intraday trading
    /// - `15min`: 15 minutes - Medium-term analysis
    /// - `1h`: 1 hour - Daily trading patterns
    /// - `2h`: 2 hours (default) - Extended market analysis
    /// - `1d`: 1 day - Long-term trends
    /// - `1w`: 1 week - Strategic market overview
    #[schema(example = "1min")]
    pub interval: Option<Interval>,

    /// Enable price routing through intermediate pairs.
    /// When true, if a direct price for the requested pair is not available,
    /// the system will attempt to calculate it using intermediate pairs.
    ///
    /// # Example
    /// For BTC/EUR when routing is enabled:
    /// - If direct BTC/EUR price is unavailable
    /// - System might route through BTC/USD and EUR/USD
    ///
    /// Default: true
    #[schema(example = true)]
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_bool_from_string")]
    pub routing: Option<bool>,

    /// Method used to aggregate prices from multiple sources.
    ///
    /// # Available modes
    /// - `median`: Middle value (default, more manipulation resistant)
    /// - `mean`: Average of all values
    /// - `twap`: Time-Weighted Average Price
    #[schema(example = "median")]
    pub aggregation: Option<AggregationMode>,

    /// Type of market entry to retrieve
    ///
    /// # Available types
    /// - `spot`: Spot market prices (default)
    /// - `perp`: Perpetual futures prices
    /// - `future`: Fixed-expiry futures prices
    #[schema(example = "spot")]
    pub entry_type: Option<EntryType>,

    /// Expiry date for future contracts in ISO 8601 format.
    /// Only applicable when `entry_type` is "future".
    ///
    /// # Example
    /// - `"2024-12-31"`: December 31, 2024 expiry
    /// - `null`: Not applicable for spot/perp markets
    #[schema(example = "2024-12-31")]
    pub expiry: Option<String>,

    /// Include source components in the response.
    /// When true, the response will include price data from individual sources.
    ///
    /// # Example
    /// - `true`: Include source breakdown in response
    /// - `false`: Return aggregated data only (default)
    #[schema(example = false)]
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_bool_from_string")]
    pub with_components: Option<bool>,
}

impl Default for GetEntryParams {
    fn default() -> Self {
        Self {
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
            interval: Some(Interval::default()),
            routing: Some(false),
            aggregation: Some(AggregationMode::default()),
            entry_type: Some(EntryType::default()),
            expiry: None,
            with_components: Some(false),
        }
    }
}

fn deserialize_bool_from_string<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    // First, try to deserialize as an Option<String>
    let opt_str = Option::<String>::deserialize(deserializer)?;

    match opt_str.as_deref() {
        Some("true") => Ok(Some(true)),
        Some("false") => Ok(Some(false)),
        Some(s) => Err(serde::de::Error::custom(format!(
            "Invalid boolean value: {s}"
        ))),
        None => Ok(None),
    }
}
