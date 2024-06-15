pub mod checkpoint_error;
pub mod currency;
pub mod currency_error;
pub mod entry;
pub mod entry_error;
pub mod publisher;
pub mod publisher_error;

type DieselResult<T> = Result<T, diesel::result::Error>;
