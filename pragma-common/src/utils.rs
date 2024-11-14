use starknet::core::types::Felt;

/// Returns a Field Element as an hexadecimal string representation.
pub fn field_element_as_hex_string(f: &Felt) -> String {
    format!("{:#x}", f)
}
