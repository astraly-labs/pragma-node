use utoipa::ToSchema;

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum ConversionError {
    #[error("failed to serialize")]
    FailedSerialization,
    #[error("invalid date time")]
    InvalidDateTime,
    #[error("failed to convert big decimal")]
    BigDecimalConversion,
    #[error("failed to convert felt")]
    FeltConversion,
    #[error("failed to convert u128")]
    U128Conversion,
    #[error("failed to convert timestamp string")]
    StringTimestampConversion,
    #[error("failed to convert price string")]
    StringPriceConversion,
}
