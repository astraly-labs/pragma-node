use pragma_entities::InfraError;
use pragma_entities::{Publishers, dto};

pub async fn get(
    pool: &deadpool_diesel::postgres::Pool,
    name: String,
) -> Result<dto::Publisher, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let res = conn
        .as_ref()
        .interact(move |conn| Publishers::get_by_name(conn, name))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)
        .map(dto::Publisher::from)?;

    Ok(res)
}
