use deadpool_diesel::InteractError;
use pragma_common::{
    timestamp::TimestampRangeError,
    types::{AggregationMode, Interval},
};
use std::{
    fmt::{self, Debug},
    num::TryFromIntError,
};
use thiserror::Error;
use utoipa::ToSchema;

use crate::models::entry_error::EntryError;

#[derive(Debug, ToSchema, thiserror::Error)]
pub enum InfraError {
    InternalServerError,
    RoutingError,
    NotFound,
    DisputerNotSet,
    SettlerNotSet,
    InvalidTimestamp(String),
    #[error(transparent)]
    #[schema(value_type = String)]
    NonZeroU32Conversion(#[from] TryFromIntError),
    #[error(transparent)]
    #[schema(value_type = String)]
    AxumError(#[from] axum::Error),
    UnsupportedInterval(Interval, AggregationMode),
}

impl InfraError {
    pub fn to_entry_error(&self, pair_id: &String) -> EntryError {
        match self {
            Self::NotFound => EntryError::NotFound(pair_id.to_string()),
            Self::RoutingError => EntryError::MissingData(pair_id.to_string()),
            Self::InvalidTimestamp(e) => {
                EntryError::InvalidTimestamp(TimestampRangeError::Other(e.to_string()))
            }
            Self::UnsupportedInterval(i, d) => EntryError::UnsupportedInterval(*i, *d),
            Self::InternalServerError
            | Self::DisputerNotSet
            | Self::SettlerNotSet
            | Self::NonZeroU32Conversion(_)
            | Self::AxumError(_) => EntryError::InternalServerError,
        }
    }
}

#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("cannot init database pool : {0}")]
    PoolDatabase(String),
    #[error("cannot find environment variable for database init : {0}")]
    VariableDatabase(String),
    #[error("database init error : {0}")]
    GenericInitDatabase(String),
    #[error("cannot init redis connection : {0}")]
    RedisConnection(String),
}

pub fn adapt_infra_error<T: Error + Debug>(error: T) -> InfraError {
    error.as_infra_error()
}

impl fmt::Display for InfraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "Not found"),
            Self::RoutingError => write!(f, "Routing Error"),
            Self::InternalServerError => write!(f, "Internal server error"),
            Self::DisputerNotSet => write!(f, "Unable to fetch disputer address"),
            Self::SettlerNotSet => write!(f, "Unable to fetch settler address"),
            Self::InvalidTimestamp(e) => write!(f, "Invalid timestamp {e}"),
            Self::NonZeroU32Conversion(e) => write!(f, "Non zero u32 conversion {e}"),
            Self::AxumError(e) => write!(f, "Axum error {e}"),
            Self::UnsupportedInterval(i, a) => {
                write!(f, "Unsupported interval {i:?} for aggregation {a:?}")
            }
        }
    }
}

pub trait Error {
    fn as_infra_error(&self) -> InfraError;
}

impl Error for diesel::result::Error {
    fn as_infra_error(&self) -> InfraError {
        match self {
            Self::NotFound => InfraError::NotFound,
            _ => InfraError::InternalServerError,
        }
    }
}

impl Error for deadpool_diesel::PoolError {
    fn as_infra_error(&self) -> InfraError {
        InfraError::InternalServerError
    }
}

impl Error for InteractError {
    fn as_infra_error(&self) -> InfraError {
        InfraError::InternalServerError
    }
}

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum RedisError {
    #[error("internal server error")]
    InternalServerError,
    #[error("could not establish a connection with Redis")]
    Connection,
    #[error("could not find option for instrument {1} at block {0}")]
    OptionNotFound(u64, String),
    #[error("merkle tree not found for block {0}")]
    MerkleTreeNotFound(u64),
    #[error("invalid option hash, could not convert to felt: {0}")]
    InvalidOptionHash(String),
    #[error("could not deserialize RawMerkleTree into MerkleTree")]
    TreeDeserialization,
    #[error("no merkle feeds published for network: {0}")]
    NoBlocks(String),
}
