#[derive(Debug)]
pub enum ConversionError {
    FailedSerialization,
    InvalidDateTime,
    BigDecimalConversion,
    FeltConversion,
    U128Conversion,
}
