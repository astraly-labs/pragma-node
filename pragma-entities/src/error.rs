use std::fmt::{self, Debug};

use deadpool_diesel::InteractError;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, ToSchema)]
pub enum InfraError {
    InternalServerError,
    NotFound,
    InvalidTimeStamp,
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
            InfraError::InvalidTimeStamp => write!(f, "Invalid timestamp")
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
