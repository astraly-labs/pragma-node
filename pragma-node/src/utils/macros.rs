#[macro_export]
macro_rules! is_enum_variant {
    ($val:ident, $var:path) => {
        match $val {
            $var { .. } => true,
            _ => false,
        }
    };
}
