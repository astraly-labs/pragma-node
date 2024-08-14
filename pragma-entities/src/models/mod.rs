pub mod checkpoint_error;
pub mod currency;
pub mod currency_error;
pub mod entries;
pub mod merkle_feed_error;
pub mod optimistic_oracle_error;
pub mod publisher;
pub mod publisher_error;

pub use entries::{entry, entry_error, future_entry};

type DieselResult<T> = Result<T, diesel::result::Error>;
