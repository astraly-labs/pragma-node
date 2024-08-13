use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;
use chrono::NaiveDateTime;
use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_monitoring::{models::OORequest, schema::oo_requests};
use diesel::prelude::*;
use diesel::dsl::*;
use diesel::pg::Pg;
#[derive(Debug, Serialize)]
pub enum Status {
    Active,
    Disputed,
    Settled,
}

#[derive(Debug, Serialize)]
pub enum SettlementResolution {
    True, 
    False,
    Undefined
}
impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status_str = match self {
            Status::Active => "Active",
            Status::Disputed => "Disputed",
            Status::Settled => "Settled",
        };
        write!(f, "{}", status_str)
    }
}

impl From<std::option::Option<bool>> for SettlementResolution {
    fn from(res: std::option::Option<bool>) -> Self {
        match res {
            Some(true) => SettlementResolution::True,
            Some(false) => SettlementResolution::False,
            None => SettlementResolution::Undefined,
        }
    }
}
impl From<bool> for SettlementResolution {
    fn from(res: bool) -> Self {
        match res {
            true => SettlementResolution::True,
            false => SettlementResolution::False,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GetAssertionsParams {
    pub status: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Assertion {
    pub assertion_id: String,
    pub claim: String,
    pub bond: BigDecimal,
    pub expiration_time: NaiveDateTime,
    pub identifier: String,
    pub status: Status,
    pub timestamp: NaiveDateTime,
}

impl fmt::Display for Assertion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Assertion {{ assertion_id: {}, claim: {}, bond: {}, expiration_time: {}, identifier: {}, status: {}, timestamp: {} }}",
            self.assertion_id,
            self.claim,
            self.bond,
            self.expiration_time,
            self.identifier,
            self.status,
            self.timestamp
        )
    }
}
#[derive(Debug, Serialize, ToSchema)]
pub struct ResolvedAssertion{
    pub assertion: Assertion, 
    pub settled_address: String, 
    pub settlement_resolution: SettlementResolution,
    pub settled_at: NaiveDateTime,
    pub disputed: bool,
    pub settlement_tx: String 
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DisputedAssertion {
    pub assertion: Assertion, 
    pub disputer: String, 
    pub disputed_at: NaiveDateTime, 
    pub disputed_tx: String, 
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GetAssertionsResponse {
    pub assertions: Vec<Assertion>,
    pub total_count: i64,
    pub current_page: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DisputeDetails {
    pub disputer_id: String,
    pub dispute_timestamp: u64,
    pub dispute_bond: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResolutionDetails {
    pub resolved_timestamp: u64,
    pub resolution: bool,
}


#[derive(Debug, Serialize, ToSchema)]
pub struct AssertionDetailsParams {
    pub assertion_id: String
}
#[derive(Debug, Serialize, ToSchema)]
pub struct AssertionDetails {
    pub assertion: Assertion,
    pub domain_id: String, 
    pub asserter: String,
    pub disputer: String,
    pub disputed: bool,
    pub callback_recipient: String,
    pub caller: String, 
    pub settled: bool,
    pub settlement_resolution: SettlementResolution,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MakeAssertionRequest {
    pub claim: String,
    pub bond: f64,
    pub expiration_time: i64,
    pub identifier: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DisputeAssertionRequest {
    pub dispute_bond: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DisputeAssertionResponse {
    pub dispute_id: String,
    pub assertion_id: String,
    pub disputer_id: String,
    pub dispute_bond: f64,
    pub dispute_timestamp: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GetDisputedAssertionsParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GetDisputedAssertionsResponse {
    pub disputed_assertions: Vec<DisputedAssertion>,
    pub total_count: usize,
    pub current_page: u32,
    pub total_pages: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GetResolvedAssertionsParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GetResolvedAssertionsResponse {
    pub resolved_assertions: Vec<ResolvedAssertion>,
    pub total_count: usize,
    pub current_page: u32,
    pub total_pages: u32,
}
