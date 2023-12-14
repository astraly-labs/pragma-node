use pragma_entities::{Publishers,NewPublisher, dto};
use pragma_entities::{adapt_infra_error, InfraError};


pub async fn _insert(
    pool: &deadpool_diesel::postgres::Pool,
    new_entry: NewPublisher,
) -> Result<dto::Publisher, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn
        .interact(move |conn| Publishers::get_by_name(conn, new_entry.name))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)
        .map(dto::Publisher::from)?;

    Ok(res)
}

pub async fn get(
    pool: &deadpool_diesel::postgres::Pool,
    name: String,
) -> Result<dto::Publisher, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn.as_ref()
        .interact(move | conn| Publishers::get_by_name(conn, name))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)
        .map(dto::Publisher::from)?;

    Ok(res)
}

pub async fn _get_all(
    pool: &deadpool_diesel::postgres::Pool,
    filter: dto::PublishersFilter,
) -> Result<Vec<dto::Publisher>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn
        .interact(move |conn| Publishers::with_filters(conn, filter))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let entries: Vec<dto::Publisher> = res
        .into_iter()
        .map(dto::Publisher::from)
        .collect();

    Ok(entries)
}
