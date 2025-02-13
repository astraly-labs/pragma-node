pub mod checkpoint;
pub mod entry;
pub mod history;
pub mod ohlc;
pub mod publisher;

use crate::{infra::repositories::entry_repository::get_interval_specifier, is_enum_variant};
use pragma_common::types::{DataType, Interval, Network};
use pragma_entities::error::InfraError;

/// Retrieve the onchain table name based on the network and data type.
pub(crate) const fn get_onchain_table_name(
    network: Network,
    data_type: DataType,
) -> Result<&'static str, InfraError> {
    let table = match (network, data_type) {
        (Network::Sepolia, DataType::SpotEntry) => "spot_entry",
        (Network::Mainnet, DataType::SpotEntry) => "mainnet_spot_entry",
        (Network::Sepolia, DataType::FutureEntry) => "future_entry",
        (Network::Mainnet, DataType::FutureEntry) => "mainnet_future_entry",
        _ => return Err(InfraError::InternalServerError),
    };
    Ok(table)
}

/// Retrieve the onchain table name for the OHLC based on network, datatype & interval.
pub(crate) fn get_onchain_ohlc_table_name(
    network: Network,
    data_type: DataType,
    interval: Interval,
) -> Result<String, InfraError> {
    let prefix_name = match (network, data_type) {
        (Network::Sepolia, DataType::SpotEntry) => "spot",
        (Network::Mainnet, DataType::SpotEntry) => "mainnet_spot",
        (Network::Sepolia, DataType::FutureEntry) => "future",
        (Network::Mainnet, DataType::FutureEntry) => "mainnet_future",
        _ => return Err(InfraError::InternalServerError),
    };
    let interval_specifier = get_interval_specifier(interval, true)?;
    let table_name = format!("{prefix_name}_{interval_specifier}_candle");
    Ok(table_name)
}

/// Retrieve the onchain table name for Timescale aggregates (medians) based on the network,
/// datatype & interval.
pub(crate) fn get_onchain_aggregate_table_name(
    network: Network,
    data_type: DataType,
    interval: Interval,
) -> Result<String, InfraError> {
    let prefix_name = match (network, data_type) {
        (Network::Sepolia, DataType::SpotEntry) => "spot_price",
        (Network::Mainnet, DataType::SpotEntry) => "mainnet_spot_price",
        (Network::Sepolia, DataType::FutureEntry) => "future_price",
        (Network::Mainnet, DataType::FutureEntry) => "mainnet_future_price",
        _ => return Err(InfraError::InternalServerError),
    };

    // NOTE: Special case because there is a mistake and we forgot the "s" on 2_hour
    let interval_specifier = if is_enum_variant!(interval, Interval::TwoHours) {
        "2_hour"
    } else {
        get_interval_specifier(interval, true)?
    };

    let table_name = format!("{prefix_name}_{interval_specifier}_agg");
    Ok(table_name)
}
