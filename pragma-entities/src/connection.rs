use crate::error::ErrorKind;
use deadpool_diesel::postgres::{Manager, Pool};

pub const ENV_ONCHAIN_DATABASE_URL: &str = "POSTGRES_DATABASE_URL";
pub const ENV_OFFCHAIN_DATABASE_URL: &str = "TIMESCALE_DATABASE_URL";
const ENV_DATABASE_MAX_CONN: &str = "DATABASE_MAX_CONN";

pub fn init_pool(app_name: &str, database_url_env: &str) -> Result<Pool, ErrorKind> {
    if database_url_env != ENV_OFFCHAIN_DATABASE_URL && database_url_env != ENV_ONCHAIN_DATABASE_URL
    {
        return Err(ErrorKind::GenericInitDatabase(format!(
            "invalid database URL environment variable: {}",
            database_url_env
        )));
    }

    let database_url = std::env::var(database_url_env)
        .map_err(|_| ErrorKind::VariableDatabase(database_url_env.to_string()))?;

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
