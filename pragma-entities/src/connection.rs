use crate::error::ErrorKind;
use deadpool_diesel::postgres::{Manager, Pool};

const ENV_DATABASE_URL: &str = "TIMESCALE_DATABASE_URL";
const ENV_DATABASE_MAX_CONN: &str = "DATABASE_MAX_CONN";

pub fn init_pool(app_name: &str) -> Result<Pool, ErrorKind> {
    let database_url = std::env::var(ENV_DATABASE_URL)
        .map_err(|_| ErrorKind::VariableDatabase(ENV_DATABASE_URL.to_string()))?;

    let database_max_conn = std::env::var(ENV_DATABASE_MAX_CONN)
        .map_err(|_| ErrorKind::VariableDatabase(ENV_DATABASE_MAX_CONN.to_string()))?
        .parse::<u32>()
        .map_err(|_| {
            ErrorKind::GenericInitDatabase(format!("cannot parse {}", ENV_DATABASE_MAX_CONN))
        })? as usize;

    let manager = Manager::new(
        format!("{}?application_name={}", database_url, app_name),
        deadpool_diesel::Runtime::Tokio1,
    );

    Pool::builder(manager)
        .max_size(database_max_conn)
        .build()
        .map_err(|e| ErrorKind::PoolDatabase(e.to_string()))
}
