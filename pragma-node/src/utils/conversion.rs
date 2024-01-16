use bigdecimal::BigDecimal;
use pragma_entities::InfraError;

pub fn convert_via_quote(
    a_price: BigDecimal,
    b_price: BigDecimal,
    output_decimals: u32,
) -> Result<BigDecimal, InfraError> {
    if b_price == BigDecimal::from(0) {
        return Err(InfraError::InternalServerError);
    }

    let power = BigDecimal::from(10_i64.pow(output_decimals));

    Ok(a_price * power / b_price)
}

pub fn normalize_to_decimals(
    value: BigDecimal,
    original_decimals: u32,
    target_decimals: u32,
) -> BigDecimal {
    if target_decimals >= original_decimals {
        let power = BigDecimal::from(10_i64.pow(target_decimals - original_decimals));
        value * power
    } else {
        let power = BigDecimal::from(10_i64.pow(original_decimals - target_decimals));
        value / power
    }
}
