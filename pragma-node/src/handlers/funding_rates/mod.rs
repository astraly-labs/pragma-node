pub mod get_instruments;
pub mod get_funding_rates;
pub mod get_historical_funding_rates;

pub use get_instruments::get_supported_instruments;
pub use get_funding_rates::get_latest_funding_rate;
pub use get_historical_funding_rates::get_historical_funding_rates;
