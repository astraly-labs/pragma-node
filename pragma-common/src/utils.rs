use starknet::core::types::Felt;

/// Returns a Field Element as an hexadecimal string representation.
pub fn field_element_as_hex_string(f: &Felt) -> String {
    format!("{:#x}", f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_element_as_hex_string() {
        let f = Felt::from(123456);
        assert_eq!(field_element_as_hex_string(&f), "0x1e240");
    }
}
