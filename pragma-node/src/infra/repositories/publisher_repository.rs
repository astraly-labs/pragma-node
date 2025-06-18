use pragma_entities::InfraError;
use pragma_entities::{Publishers, dto};

pub async fn get(
    pool: &deadpool_diesel::postgres::Pool,
    name: String,
) -> Result<dto::Publisher, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let res = conn
        .as_ref()
        .interact(move |conn| Publishers::get_by_name_transactional(conn, name))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)
        .map(dto::Publisher::from)?;

    Ok(res)
}

pub async fn get_with_filters(
    pool: &deadpool_diesel::postgres::Pool,
    filters: dto::PublishersFilter,
) -> Result<Vec<dto::Publisher>, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let res = conn
        .as_ref()
        .interact(move |conn| Publishers::with_filters_transactional(conn, filters))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    let publishers = res.into_iter().map(dto::Publisher::from).collect();
    Ok(publishers)
}

pub async fn get_account_address_by_name(
    pool: &deadpool_diesel::postgres::Pool,
    name: String,
) -> Result<String, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let res = conn
        .as_ref()
        .interact(move |conn| Publishers::get_account_address_by_name_transactional(conn, name))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(res)
}
