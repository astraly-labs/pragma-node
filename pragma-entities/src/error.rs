use deadpool_diesel::InteractError;
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
    NotFound,
    InvalidTimeStamp,
    #[error(transparent)]
    NonZeroU32Conversion(#[from] TryFromIntError),
    #[error(transparent)]
    AxumError(#[from] axum::Error),
}

impl InfraError {
    pub fn to_entry_error(&self, pair_id: &String) -> EntryError {
        match self {
            InfraError::InternalServerError => EntryError::InternalServerError,
            InfraError::NotFound => EntryError::NotFound(pair_id.to_string()),
            InfraError::InvalidTimeStamp => EntryError::InvalidTimestamp,
            InfraError::NonZeroU32Conversion(_) => EntryError::InternalServerError,
            InfraError::AxumError(_) => EntryError::InternalServerError,
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
}

pub fn adapt_infra_error<T: Error + Debug>(error: T) -> InfraError {
    println!("Error: {:?}", error);
    error.as_infra_error()
}

impl fmt::Display for InfraError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InfraError::NotFound => write!(f, "Not found"),
            InfraError::InternalServerError => write!(f, "Internal server error"),
            InfraError::InvalidTimeStamp => write!(f, "Invalid timestamp"),
            InfraError::NonZeroU32Conversion(e) => write!(f, "Non zero u32 conversion {e}"),
            InfraError::AxumError(e) => write!(f, "Axum error {e}"),
        }
    }
}

pub trait Error {
    fn as_infra_error(&self) -> InfraError;
}

impl Error for diesel::result::Error {
    fn as_infra_error(&self) -> InfraError {
        match self {
            diesel::result::Error::NotFound => InfraError::NotFound,
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
