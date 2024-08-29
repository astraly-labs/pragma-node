use crate::handlers::optimistic_oracle::types::{
    Assertion, AssertionDetails, DisputedAssertion, ResolvedAssertion, Status,
};
#[allow(unused_imports)]
use diesel::prelude::*;
use diesel::sql_types::Bool;
use pragma_entities::models::optimistic_oracle_error::OptimisticOracleError;
use pragma_monitoring::{models::OORequest, schema::oo_requests};

// if no status provided, returns the list of all the available assertions
pub async fn get_assertions(
    onchain_pool: &deadpool_diesel::postgres::Pool,
    status: Option<String>,
    page: u32,
    limit: u32,
) -> Result<Vec<Assertion>, OptimisticOracleError> {
    let conn = onchain_pool
        .get()
        .await
        .map_err(|_| OptimisticOracleError::DatabaseConnection)?;

    let status_clone = status.clone();

    let results: Vec<OORequest> = conn
        .interact(move |conn| {
            let mut query = oo_requests::table.into_boxed();

            if let Some(status) = status_clone {
                match status.as_str() {
                    "settled" => query = query.filter(oo_requests::settled.eq(Some(true))),
                    "disputed" => query = query.filter(oo_requests::disputed.eq(Some(true))),
                    "active" => {
                        query = query.filter(
                            oo_requests::settled
                                .is_null()
                                .and(oo_requests::disputed.is_null()),
                        )
                    }
                    _ => {}
                }
            };

            query = query.filter(diesel::dsl::sql::<Bool>("upper(_cursor) IS NULL"));

            query
                .offset(((page - 1) * limit) as i64)
                .limit(limit as i64)
                .load(conn)
                .map_err(|_| OptimisticOracleError::DatabaseConnection)
        })
        .await
        .map_err(|_| OptimisticOracleError::DatabaseConnection)??;

    let assertions: Vec<Assertion> = results
        .into_iter()
        .map(|request| Assertion {
            assertion_id: request.assertion_id.to_string(),
            claim: request.claim,
            bond: request.bond,
            expiration_time: request.expiration_timestamp,
            identifier: request.identifier,
            status: get_status(request.disputed, request.settled),
            timestamp: request.updated_at,
            currency: request.currency,
        })
        .collect();

    Ok(assertions)
}

// Function to get assertion details
pub async fn get_assertion_details(
    onchain_pool: &deadpool_diesel::postgres::Pool,
    assertion_id: &str,
) -> Result<AssertionDetails, OptimisticOracleError> {
    let conn = onchain_pool
        .get()
        .await
        .map_err(|_| OptimisticOracleError::DatabaseConnection)?;

    let assertion_id = assertion_id.to_string();

    let request: OORequest = conn
        .interact(move |conn| {
            oo_requests::table
                .filter(diesel::dsl::sql::<Bool>("upper(_cursor) IS NULL"))
                .filter(oo_requests::assertion_id.eq(&assertion_id))
                .first(conn)
                .map_err(|_| OptimisticOracleError::AssertionDetailsIssue(assertion_id))
        })
        .await
        .map_err(|_| OptimisticOracleError::DatabaseConnection)??;

    let status = get_status(request.disputed, request.settled);
    Ok(AssertionDetails {
        assertion: Assertion {
            assertion_id: request.assertion_id.to_string(),
            claim: request.claim,
            bond: request.bond,
            expiration_time: request.expiration_timestamp,
            identifier: request.identifier,
            status,
            timestamp: request.updated_at,
            currency: request.currency,
        },
        domain_id: request.domain_id,
        asserter: request.asserter,
        disputer: request.disputer.unwrap_or("None".to_string()),
        disputed: request.disputed.unwrap_or(false),
        callback_recipient: request.callback_recipient,
        dispute_id: request.dispute_id.unwrap_or("None".to_string()),
        caller: request.caller,
        settled: request.settled.unwrap_or(false),
        settle_caller: request.settle_caller.unwrap_or("None".to_string()),
        settlement_resolution: request.settlement_resolution.into(),
    })
}

// Function to get disputed assertions
pub async fn get_disputed_assertions(
    onchain_pool: &deadpool_diesel::postgres::Pool,
    page: u32,
    limit: u32,
) -> Result<Vec<DisputedAssertion>, OptimisticOracleError> {
    let conn = onchain_pool
        .get()
        .await
        .map_err(|_| OptimisticOracleError::DatabaseConnection)?;

    let results: Vec<OORequest> = conn
        .interact(move |conn| {
            let query = oo_requests::table
                .filter(diesel::dsl::sql::<Bool>("upper(_cursor) IS NULL"))
                .filter(oo_requests::disputed.eq(true))
                .offset(((page - 1) * limit) as i64)
                .limit(limit as i64);

            query
                .load(conn)
                .map_err(|_| OptimisticOracleError::DatabaseConnection)
        })
        .await
        .map_err(|_| OptimisticOracleError::DatabaseConnection)??;

    results
        .into_iter()
        .map(|request| {
            let disputer = request.disputer.ok_or_else(|| {
                OptimisticOracleError::DisputerNotSet(request.assertion_id.clone())
            })?;

            Ok(DisputedAssertion {
                assertion: Assertion {
                    assertion_id: request.assertion_id.to_string(),
                    claim: request.claim,
                    bond: request.bond,
                    expiration_time: request.expiration_timestamp,
                    identifier: request.identifier,
                    status: Status::Disputed,
                    timestamp: request.updated_at,
                    currency: request.currency
                },
                disputer,
                disputed_at: request.updated_at,
                disputed_tx: request.updated_at_tx,
                dispute_id: request.dispute_id.unwrap_or("None".to_string()),
            })
        })
        .collect()
}

// Function to get resolved assertions
pub async fn get_resolved_assertions(
    onchain_pool: &deadpool_diesel::postgres::Pool,
    page: u32,
    limit: u32,
) -> Result<Vec<ResolvedAssertion>, OptimisticOracleError> {
    let conn = onchain_pool
        .get()
        .await
        .map_err(|_| OptimisticOracleError::DatabaseConnection)?;

    let results: Vec<OORequest> = conn
        .interact(move |conn| {
            let query = oo_requests::table
                .filter(diesel::dsl::sql::<Bool>("upper(_cursor) IS NULL"))
                .filter(oo_requests::settled.eq(true))
                .offset(((page - 1) * limit) as i64)
                .limit(limit as i64);

            query
                .load(conn)
                .map_err(|_| OptimisticOracleError::DatabaseConnection)
        })
        .await
        .map_err(|_| OptimisticOracleError::DatabaseConnection)??;

    results
        .into_iter()
        .map(|request| {
            let settled_address = request.settle_caller.clone().ok_or_else(|| {
                OptimisticOracleError::SettlerNotSet(request.assertion_id.clone())
            })?;

            Ok(ResolvedAssertion {
                assertion: Assertion {
                    assertion_id: request.assertion_id,
                    claim: request.claim,
                    bond: request.bond,
                    expiration_time: request.expiration_timestamp,
                    identifier: request.identifier,
                    status: Status::Settled,
                    timestamp: request.updated_at,
                    currency:request.currency
                },
                settled_address,
                settlement_resolution: request.settlement_resolution.into(),
                disputed: request.disputed.unwrap_or(false),
                settled_at: request.updated_at,
                settle_caller: request.settle_caller.unwrap_or("None".to_string()),
                dispute_id: request.dispute_id.unwrap_or("None".to_string()),
                settlement_tx: request.updated_at_tx,
            })
        })
        .collect()
}

fn get_status(disputed: Option<bool>, settled: Option<bool>) -> Status {
    match (disputed, settled) {
        (Some(true), _) => Status::Disputed,
        (_, Some(true)) => Status::Settled,
        _ => Status::Active,
    }
}
