use diesel::prelude::*;
use diesel::sql_types::{Bool, Integer, Text, BigInt};
use crate::AppState;
use bigdecimal::{BigDecimal, ToPrimitive, Zero};
use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_monitoring::{models::OORequest, schema::oo_requests};
use std::fmt;
use diesel::deserialize::{self, FromSql};
use crate::handlers::optimistic_oracle::types::{Assertion,SettlementResolution, Status,AssertionDetails,ResolutionDetails, DisputeDetails,DisputeAssertionResponse,ResolvedAssertion,DisputedAssertion};
use chrono::Utc;
use diesel::dsl::count_star;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_query;


// if no status provided, returns the list of all the available assertions 
pub async fn get_assertions(
    onchain_pool: &deadpool_diesel::postgres::Pool,
    status: Option<String>,
    page: u32,
    limit: u32,
) -> Result<Vec<Assertion>, InfraError> {
    let conn = onchain_pool.get().await.map_err(adapt_infra_error)?;


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
        // Execute count query
        // let total_count: i64 = oo_requests::table
        //     .select(count_star())
        //     .first(conn)
        //     .map_err(adapt_infra_error)?;
        
        let results: Vec<OORequest> = query
            .offset(((page - 1) * limit) as i64)
            .limit(limit as i64)
            .load(conn)
            .map_err(|e| adapt_infra_error(e))?;

        let assertions: Vec<Assertion> = results
            .into_iter()
            .map(|request| Assertion {
                assertion_id: request.assertion_id.to_string(),
                claim: request.claim,
                bond: request.bond,
                expiration_time: request.expiration_timestamp,
                identifier: request.identifier,
                status: match (request.disputed, request.settled) {
                    (Some(true), _) => Status::Disputed,           // Disputed if `disputed` is `true`
                    (_, Some(true)) => Status::Settled,            // Settled if `settled` is `true`
                    _ => Status::Active,                           // Active if neither are `true`, or both are `None`
                },
                timestamp: request.updated_at,
            })
            .collect();

        Ok(assertions)
    })
    .await
    .map_err(adapt_infra_error)?
}

// Function to get assertion details
pub async fn get_assertion_details(
    onchain_pool: &deadpool_diesel::postgres::Pool,
    assertion_id: &str,
) -> Result<AssertionDetails, InfraError> {
    let conn = onchain_pool.get().await.map_err(adapt_infra_error)?;

    let assertion_id = assertion_id.to_string();

    conn.interact(move |conn| {
        let request: OORequest = oo_requests::table
            .filter(diesel::dsl::sql::<Bool>("upper(_cursor) IS NULL"))
            .filter(oo_requests::assertion_id.eq(&assertion_id))
            .first(conn)
            .map_err(adapt_infra_error)?;

        let status = match (request.disputed, request.settled) {
            (Some(true), _) => Status::Disputed,
            (_, Some(true)) => Status::Settled,
            _ => Status::Active,
        };


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
    .map_err(adapt_infra_error)?
}


// Function to get disputed assertions
pub async fn get_disputed_assertions(
    onchain_pool: &deadpool_diesel::postgres::Pool,
    page: u32,
    limit: u32,
) -> Result<Vec<DisputedAssertion>, InfraError> {
    let conn = onchain_pool.get().await.map_err(adapt_infra_error)?;

    conn.interact(move |conn| {
        let query = oo_requests::table
            .filter(diesel::dsl::sql::<Bool>("upper(_cursor) IS NULL"))
            .filter(oo_requests::disputed.eq(true))
            .offset(((page - 1) * limit) as i64)
            .limit(limit as i64);

        let results: Vec<OORequest> = query.load(conn).map_err(adapt_infra_error)?;

        let disputed_assertions: Vec<DisputedAssertion> = results
            .into_iter()
            .map(|request| {
                // You'll need to implement logic to fetch dispute details
                let dispute_details = DisputeDetails {
                    disputer_id: "placeholder_disputer_id".to_string(),
                    dispute_timestamp: 0, // Placeholder
                    dispute_bond: 0.0, // Placeholder
                };

                DisputedAssertion {
                    assertion: Assertion {assertion_id: request.assertion_id.to_string(),
                        claim: request.claim,
                        bond: request.bond,
                        expiration_time: request.expiration_timestamp,
                        identifier: request.identifier,
                        status: Status::Disputed,
                        timestamp: request.updated_at,
                    }, 
                disputer: request.disputer.expect("Unable to fetch disputer address"), 
                disputed_at: request.updated_at, 
                disputed_tx: request.updated_at_tx
            }
            })
            .collect();

        Ok(disputed_assertions)
    })
    .await
    .map_err(adapt_infra_error)?
}

// Function to get resolved assertions
pub async fn get_resolved_assertions(
    onchain_pool: &deadpool_diesel::postgres::Pool,
    page: u32,
    limit: u32,
) -> Result<Vec<ResolvedAssertion>, InfraError> {
    let conn = onchain_pool.get().await.map_err(adapt_infra_error)?;

    conn.interact(move |conn| {
        let query = oo_requests::table
            .filter(diesel::dsl::sql::<Bool>("upper(_cursor) IS NULL"))
            .filter(oo_requests::settled.eq(true))
            .offset(((page - 1) * limit) as i64)
            .limit(limit as i64);


        let results: Vec<OORequest> = query.load(conn).map_err(adapt_infra_error)?;

        let resolved_assertions: Vec<ResolvedAssertion> = results
            .into_iter()
            .map(|request| {
                // You'll need to implement logic to fetch resolution details
                let resolution_details = ResolutionDetails {
                    resolved_timestamp: 0, // Placeholder
                    resolution: true, // Placeholder
                };

                ResolvedAssertion {
                    assertion: Assertion {                    
                        assertion_id: request.assertion_id,
                        claim: request.claim,
                        bond: request.bond.into(),
                        expiration_time: request.expiration_timestamp,
                        identifier: request.identifier,
                        status: Status::Settled,
                        timestamp: request.updated_at,
                    }, 
                    settled_address: request.settle_caller.expect("Unable to fetch settler address"), 
                    settlement_resolution: request.settlement_resolution.into(),
                    disputed: request.disputed.unwrap_or(false),
                    settled_at: request.updated_at,
                    settlement_tx: request.updated_at_tx,
                }
            })
            .collect();

        Ok(resolved_assertions)
    })
    .await
    .map_err(adapt_infra_error)?
}