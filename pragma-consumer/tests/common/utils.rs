use std::cmp::Ordering;

use starknet::core::{crypto::pedersen_hash, types::FieldElement};

// Utility hash function from the MerkleTree struct.
pub fn hash(a: &FieldElement, b: &FieldElement) -> FieldElement {
    let (a_sorted, b_sorted) = match a.cmp(b) {
        Ordering::Less | Ordering::Equal => (a, b),
        Ordering::Greater => (b, a),
    };
    pedersen_hash(a_sorted, b_sorted)
}
