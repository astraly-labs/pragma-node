use pragma_entities::{adapt_infra_error, InfraError};
use pragma_entities::{dto, Publishers};

pub async fn get(
    pool: &deadpool_diesel::postgres::Pool,
    name: String,
) -> Result<dto::Publisher, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn
        .as_ref()
        .interact(move |conn| Publishers::get_by_name(conn, name))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)
        .map(dto::Publisher::from)?;

    Ok(res)
}
