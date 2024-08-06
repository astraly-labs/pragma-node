pub mod entries;
pub mod hex_hash;
pub mod pricer;
pub mod timestamp;
pub mod ws;

#[macro_export]
macro_rules! is_enum_variant {
    ($val:ident, $var:path) => {
        match $val {
            $var { .. } => true,
            _ => false,
        }
    };
}
