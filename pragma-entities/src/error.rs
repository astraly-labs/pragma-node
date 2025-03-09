use deadpool_diesel::{InteractError, PoolError};
use pragma_common::{
    timestamp::TimestampError,
    types::{AggregationMode, Interval, Network},
};
use std::{
    fmt::{self, Debug},
    num::TryFromIntError,
};
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error, ToSchema)]
pub enum WebSocketError {
    #[error("could not create a channel with the client")]
    ChannelInit,
    #[error("could not decode client message: {0}")]
    MessageDecode(String),
    #[error("could not close the channel")]
    ChannelClose,
}

#[derive(Debug, Error)]
pub enum PragmaNodeError {
    #[error("cannot init database pool : {0}")]
    PoolDatabase(String),
    #[error("cannot find environment variable for database init : {0}")]
    MissingDbEnvVar(String),
    #[error("database init error : {0}")]
    GenericInitDatabase(String),
    #[error("cannot init redis connection : {0}")]
    RedisConnection(String),
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

#[derive(Debug, thiserror::Error)]
pub enum InfraError {
    // Bad request (400)
    InvalidTimestamp(TimestampError),
    UnsupportedInterval(Interval, AggregationMode),
    // Not Found error (404)
    RoutingError(String),
    EntryNotFound(String),
    PairNotFound(String),
    CheckpointNotFound(String),
    PublishersNotFound,
    // Specifics for Optimistic Oracle
    DisputerNotSet,
    SettlerNotSet,
    // Known internal errors
    #[error(transparent)]
    NonZeroU32Conversion(#[from] TryFromIntError),
    #[error(transparent)]
    AxumError(#[from] axum::Error),
    DbPoolError(#[from] PoolError),
    DbInteractionError(#[from] InteractError),
    DbResultError(#[from] diesel::result::Error),
    NoRpcAvailable(Network),
    // Unknown internal Server Error - should never be shown to the user
    InternalServerError,
    WebSocketError(#[from] WebSocketError),
}

impl fmt::Display for InfraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // 400
            Self::InvalidTimestamp(e) => write!(f, "Invalid timestamp: {e}"),
            Self::UnsupportedInterval(i, a) => {
                write!(f, "Unsupported interval {i:?} for aggregation {a:?}")
            }
            // 404
            Self::EntryNotFound(pair_id) => write!(f, "Entry not found for pair {pair_id}"),
            Self::PairNotFound(pair_id) => write!(f, "Pair {pair_id} not found"),
            Self::RoutingError(pair_id) => write!(f, "No route found for {pair_id}"),
            Self::CheckpointNotFound(pair_id) => write!(f, "No checkpoint found for {pair_id}"),
            Self::PublishersNotFound => write!(f, "No publishers found"),
            // 500
            Self::DbResultError(e) => write!(f, "Error fetching from db {e}"),
            Self::DbInteractionError(e) => write!(f, "Error querying from db {e}"),
            Self::DbPoolError(e) => write!(f, "Error connecting to db {e}"),
            Self::AxumError(e) => write!(f, "Axum error {e}"),
            Self::NoRpcAvailable(network) => write!(f, "No RPC available for network {network}"),
            Self::NonZeroU32Conversion(e) => write!(f, "Non zero u32 conversion {e}"),
            Self::InternalServerError => write!(f, "Internal server error"),
            // Unclassified
            Self::DisputerNotSet => write!(f, "Unable to fetch disputer address"),
            Self::SettlerNotSet => write!(f, "Unable to fetch settler address"),
            Self::WebSocketError(e) => write!(f, "WebSocket error {e}"),
        }
    }
}
