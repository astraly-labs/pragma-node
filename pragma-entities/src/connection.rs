use crate::error::PragmaNodeError;
use deadpool_diesel::postgres::{Manager, Pool};

pub const ENV_ONCHAIN_DATABASE_URL: &str = "ONCHAIN_DATABASE_URL";
pub const ENV_OFFCHAIN_DATABASE_URL: &str = "OFFCHAIN_DATABASE_URL";
const ENV_DATABASE_MAX_CONN: &str = "DATABASE_MAX_CONN";

pub fn init_pool(app_name: &str, database_url_env: &str) -> Result<Pool, PragmaNodeError> {
    if database_url_env != ENV_OFFCHAIN_DATABASE_URL && database_url_env != ENV_ONCHAIN_DATABASE_URL
    {
        return Err(PragmaNodeError::GenericInitDatabase(format!(
            "invalid database URL environment variable: {database_url_env}",
        )));
    }

    let database_url = std::env::var(database_url_env)
        .map_err(|_| PragmaNodeError::MissingDbEnvVar(database_url_env.to_string()))?;

    let database_max_conn = std::env::var(ENV_DATABASE_MAX_CONN)
        .map_err(|_| PragmaNodeError::MissingDbEnvVar(ENV_DATABASE_MAX_CONN.to_string()))?
        .parse::<u32>()
        .map_err(|_| {
            PragmaNodeError::GenericInitDatabase(format!("cannot parse {ENV_DATABASE_MAX_CONN}"))
        })? as usize;

    let manager = Manager::new(
        format!("{database_url}?application_name={app_name}"),
        deadpool_diesel::Runtime::Tokio1,
    );

    Pool::builder(manager)
        .max_size(database_max_conn)
        .build()
        .map_err(|e| PragmaNodeError::PoolDatabase(e.to_string()))
}
