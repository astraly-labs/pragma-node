use bigdecimal::BigDecimal;
use bigdecimal::num_bigint::ToBigInt;

pub trait HexFormat {
    fn to_hex_string(&self) -> String;
}

impl HexFormat for BigDecimal {
    fn to_hex_string(&self) -> String {
        let bigint = self.to_bigint().unwrap_or_default();

        format!("0x{bigint:x}")
    }
}
