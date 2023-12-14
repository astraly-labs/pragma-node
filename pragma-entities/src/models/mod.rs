pub mod currency;
pub mod entry;
pub mod publisher;
pub mod currency_error;
pub mod entry_error;
pub mod publisher_error;

type DieselResult<T> = Result<T, diesel::result::Error>;