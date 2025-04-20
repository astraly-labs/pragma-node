pub mod checkpoint_error;
pub mod entries;
pub mod funding_rate;
pub mod publisher;
pub mod publisher_error;

pub use entries::{entry, entry_error, future_entry};

type DieselResult<T> = Result<T, diesel::result::Error>;
