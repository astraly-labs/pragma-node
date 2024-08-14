use diesel::prelude::*;
use diesel::sql_types::Bool;
use pragma_monitoring::{models::OORequest, schema::oo_requests};
use crate::handlers::optimistic_oracle::types::{Assertion, Status,AssertionDetails,ResolvedAssertion,DisputedAssertion};
use pragma_entities::models::optimistic_oracle_error::OptimisticOracleError;



// if no status provided, returns the list of all the available assertions 
pub async fn get_assertions(
    onchain_pool: &deadpool_diesel::postgres::Pool,
    status: Option<String>,
    page: u32,
    limit: u32,
) -> Result<Vec<Assertion>, OptimisticOracleError> {
    let conn = onchain_pool.get().await.map_err(|_| OptimisticOracleError::DatabaseConnection)?;


    conn.interact(move |conn| {
        let mut query = oo_requests::table.into_boxed();

        // Apply status filter if provided
       if let Some(status) = status {
            match status.as_str() {
                "settled" => {query = query.filter(oo_requests::settled.eq(Some(true)))}, 
                "disputed" => {query= query.filter(oo_requests::disputed.eq(Some(true)))}, 
                "active" => {query= query.filter(oo_requests::settled.is_null().and(oo_requests::disputed.is_null()))},
                _ => {}
            }
        };

        query = query.filter(diesel::dsl::sql::<Bool>("upper(_cursor) IS NULL"));
       
        
        let results: Vec<OORequest> = query
            .offset(((page - 1) * limit) as i64)
            .limit(limit as i64)
            .load(conn)
            .map_err(|_|OptimisticOracleError::DatabaseConnection)?;

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
            })
            .collect();

        Ok(assertions)
    })
    .await
    .map_err(|_| OptimisticOracleError::DatabaseConnection)?
}

// Function to get assertion details
pub async fn get_assertion_details(
    onchain_pool: &deadpool_diesel::postgres::Pool,
    assertion_id: &str,
) -> Result<AssertionDetails, OptimisticOracleError> {
    let conn = onchain_pool.get().await.map_err(|_| OptimisticOracleError::DatabaseConnection)?;

    let assertion_id = assertion_id.to_string();

    conn.interact(move |conn| {
        let request: OORequest = oo_requests::table
            .filter(diesel::dsl::sql::<Bool>("upper(_cursor) IS NULL"))
            .filter(oo_requests::assertion_id.eq(&assertion_id))
            .first(conn)
            .map_err(|_| OptimisticOracleError::AssertionDetailsIssue(assertion_id))?;

        let status = get_status(request.disputed, request.settled);
        Ok(AssertionDetails {
            assertion: Assertion{
                assertion_id: request.assertion_id.to_string(),
                claim: request.claim,
                bond: request.bond,
                expiration_time: request.expiration_timestamp,
                identifier: request.identifier,
                status:status,
                timestamp: request.updated_at,
            },
            domain_id: request.domain_id,
            asserter: request.asserter,
            disputer: request.disputer.unwrap_or("None".to_string()),
            disputed: request.disputed.unwrap_or(false),
            callback_recipient: request.callback_recipient,
            caller: request.caller, 
            settled: request.settled.unwrap_or(false),
            settlement_resolution: request.settlement_resolution.into(),
        })
    })
    .await
    .map_err(|_| OptimisticOracleError::DatabaseConnection)?
}

pub async fn get_disputed_assertions(
    onchain_pool: &deadpool_diesel::postgres::Pool,
    page: u32,
    limit: u32,
) -> Result<Vec<DisputedAssertion>, OptimisticOracleError> {
    let conn = onchain_pool.get().await.map_err(|_| OptimisticOracleError::DatabaseConnection)?;

    conn.interact(move |conn| {
        let query = oo_requests::table
            .filter(diesel::dsl::sql::<Bool>("upper(_cursor) IS NULL"))
            .filter(oo_requests::disputed.eq(true))
            .offset(((page - 1) * limit) as i64)
            .limit(limit as i64);

        let results: Vec<OORequest> = query.load(conn).map_err(|_| OptimisticOracleError::DatabaseConnection)?;

        let disputed_assertions: Result<Vec<DisputedAssertion>, OptimisticOracleError> = results
            .into_iter()
            .map(|request| {
                let disputer = request.disputer
                    .ok_or_else(|| OptimisticOracleError::DisputerNotSet(request.assertion_id.clone()))?;

                Ok(DisputedAssertion {
                    assertion: Assertion {
                        assertion_id: request.assertion_id.to_string(),
                        claim: request.claim,
                        bond: request.bond,
                        expiration_time: request.expiration_timestamp,
                        identifier: request.identifier,
                        status: Status::Disputed,
                        timestamp: request.updated_at,
                    },
                    disputer,
                    disputed_at: request.updated_at,
                    disputed_tx: request.updated_at_tx
                })
            })
            .collect();

        disputed_assertions
    })
    .await
    .map_err(|_| OptimisticOracleError::DatabaseConnection)?
}

// Function to get resolved assertions
pub async fn get_resolved_assertions(
    onchain_pool: &deadpool_diesel::postgres::Pool,
    page: u32,
    limit: u32,
) -> Result<Vec<ResolvedAssertion>, OptimisticOracleError> {
    let conn = onchain_pool.get().await.map_err(|_| OptimisticOracleError::DatabaseConnection)?;

    conn.interact(move |conn| {
        let query = oo_requests::table
            .filter(diesel::dsl::sql::<Bool>("upper(_cursor) IS NULL"))
            .filter(oo_requests::settled.eq(true))
            .offset(((page - 1) * limit) as i64)
            .limit(limit as i64);

        let results: Vec<OORequest> = query.load(conn).map_err(|_| OptimisticOracleError::DatabaseConnection)?;

        let resolved_assertions: Result<Vec<ResolvedAssertion>, OptimisticOracleError> = results
            .into_iter()
            .map(|request| {
                let settled_address = request.settle_caller
                    .ok_or_else(|| OptimisticOracleError::SettlerNotSet(request.assertion_id.clone()))?;

                Ok(ResolvedAssertion {
                    assertion: Assertion {                    
                        assertion_id: request.assertion_id,
                        claim: request.claim,
                        bond: request.bond.into(),
                        expiration_time: request.expiration_timestamp,
                        identifier: request.identifier,
                        status: Status::Settled,
                        timestamp: request.updated_at,
                    }, 
                    settled_address,
                    settlement_resolution: request.settlement_resolution.into(),
                    disputed: request.disputed.unwrap_or(false),
                    settled_at: request.updated_at,
                    settlement_tx: request.updated_at_tx,
                })
            })
            .collect();

        resolved_assertions
    })
    .await
    .map_err(|_| OptimisticOracleError::DatabaseConnection)?
}

fn get_status(disputed: Option<bool>, settled:Option<bool>) -> Status{
    match (disputed, settled) {
        (Some(true), _) => Status::Disputed,
        (_, Some(true)) => Status::Settled,
        _ => Status::Active,
    }
}