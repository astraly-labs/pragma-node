pub mod connection;
pub mod db;
pub mod dto;
pub mod error;
pub mod macros;
pub mod models;
pub mod schema;

pub use models::entries::entry_error::EntryError;

// exporting for idiomatic use
pub use error::{adapt_infra_error, InfraError};
pub use models::{
    checkpoint_error::CheckpointError,
    currency::Currency,
    entry::{Entry, NewEntry},
    entry_error::VolatilityError,
    future_entry::{FutureEntry, NewFutureEntry},
    publisher::{NewPublisher, Publishers},
    publisher_error::PublisherError,
};
