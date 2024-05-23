use bigdecimal::ToPrimitive;
use starknet::core::utils::cairo_short_string_to_felt;

use crate::config::network::NetworkConfig;

use pragma_entities::InfraError;
use starknet::{
    core::types::{BlockId, BlockTag, FieldElement, FunctionCall},
    macros::selector,
    providers::Provider,
};

pub struct GetDataMedianResponse {
    pub price: f64,
    pub decimals: u8,
    pub last_updated_timestamp: u64,
    pub num_sources_aggregated: u32,
    pub expiration_timestamp: u64,
}

pub async fn get_data_median(
    config: NetworkConfig,
    pair_id: String,
) -> Result<GetDataMedianResponse, InfraError> {
    let client: std::sync::Arc<
        starknet::providers::JsonRpcClient<starknet::providers::jsonrpc::HttpTransport>,
    > = config.provider();

    let pair_key = cairo_short_string_to_felt(&pair_id).expect("failed to convert pair id");
    let calldata = vec![FieldElement::ZERO, pair_key];
    let res = client
        .call(
            FunctionCall {
                contract_address: config.oracle_address,
                entry_point_selector: selector!("get_data_median"),
                calldata,
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        // TODO(akhercha): Handle error properly
        .map_err(|_| InfraError::InternalServerError)?;

    // TODO(akhercha): Parse the response way better + handle on chain errors
    let price = res
        .first()
        .ok_or(InfraError::InternalServerError)?
        .to_big_decimal(8)
        .to_f64()
        .expect("failed to convert price to f64");
    let decimals = res
        .get(1)
        .ok_or(InfraError::InternalServerError)?
        .to_big_decimal(0)
        .to_u8()
        .expect("failed to convert decimals to u8");
    let last_updated_timestamp = res
        .get(2)
        .ok_or(InfraError::InternalServerError)?
        .to_big_decimal(0)
        .to_u64()
        .expect("failed to convert last_updated_timestamp to u64");
    let num_sources_aggregated = res
        .get(3)
        .ok_or(InfraError::InternalServerError)?
        .to_big_decimal(0)
        .to_u32()
        .expect("failed to convert num_sources_aggregated to u32");
    let expiration_timestamp = res
        .get(4)
        .ok_or(InfraError::InternalServerError)?
        .to_big_decimal(0)
        .to_u64()
        .expect("failed to convert expiration_timestamp to u64");

    Ok(GetDataMedianResponse {
        price,
        decimals,
        last_updated_timestamp,
        num_sources_aggregated,
        expiration_timestamp,
    })
}
