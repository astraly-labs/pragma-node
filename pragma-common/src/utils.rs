use starknet::core::types::FieldElement;

/// Returns a Field Element as an hexadecimal string representation.
pub fn field_element_as_hex_string(f: &FieldElement) -> String {
    format!("{:#x}", f)
}
