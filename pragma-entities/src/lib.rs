pub mod connection;
pub mod db;
pub mod dto;
pub mod error;
pub mod models;
pub mod schema;

// exporting for idiomatic use
pub use error::{adapt_infra_error, InfraError};
pub use models::{
    currency::Currency,
    currency_error::CurrencyError,
    entry::{Entry, NewEntry},
    entry_error::{EntryError, VolatilityError},
    publisher::{NewPublisher, Publishers},
    publisher_error::PublisherError,
};
