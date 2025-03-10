pub mod create_entry;
pub mod create_future_entry;
pub mod get_entry;
pub mod get_expiries;
pub mod get_ohlc;
pub mod get_volatility;
pub mod onchain;
pub mod optimistic_oracle;
pub mod publish_entry_ws;
pub mod stream;
pub mod subscribe_to_entry;
pub mod subscribe_to_price;

pub use create_entry::create_entries;
pub use create_future_entry::create_future_entries;
pub use get_entry::get_entry;
pub use get_expiries::get_expiries;
pub use get_ohlc::get_ohlc;
pub use get_volatility::get_volatility;
pub use publish_entry_ws::publish_entry;
pub use subscribe_to_entry::subscribe_to_entry;
pub use subscribe_to_price::subscribe_to_price;

use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use pragma_common::types::{AggregationMode, DataType, Interval};

use pragma_common::types::timestamp::UnixTimestamp;

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

impl From<EntryType> for DataType {
    fn from(value: EntryType) -> Self {
        match value {
            EntryType::Spot => Self::SpotEntry,
            EntryType::Future => Self::FutureEntry,
            EntryType::Perp => Self::PerpEntry,
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
    /// - `1647820800`: Returns price data from March 21, 2022 00:00:00 UTC
    /// - `null`: Returns the most recent price update
    #[schema(value_type = i64, example = 1647820800)]
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
    /// - System might route through BTC/USD and EUR/EUR
    ///
    /// Default: true
    #[schema(example = true)]
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
    /// Only applicable when entry_type is "future".
    ///
    /// # Example
    /// - `"2024-12-31"`: December 31, 2024 expiry
    /// - `null`: Not applicable for spot/perp markets
    #[schema(example = "2024-12-31")]
    pub expiry: Option<String>,
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
        }
    }
}
