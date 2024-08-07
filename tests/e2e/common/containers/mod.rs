use testcontainers_modules::postgres::Postgres;

pub mod offchain_db;
pub mod onchain_db;
pub mod pragma_node;
pub mod utils;

// Postgres from testcontainers-modules works the same as Timescale.
// Instead of creating a whole new Image we just use this one but rename it
// timescale in our test suite.
pub type Timescale = Postgres;
