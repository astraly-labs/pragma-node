use std::cmp::Ordering;

use starknet::core::{crypto::pedersen_hash, types::FieldElement};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MerkleTreeError {
    #[error("cannot build a merkle tree from empty leaves")]
    EmptyLeaves,
}

/// Simple MerkleTree.
/// Reference:
/// https://github.com/software-mansion/starknet.py/blob/development/starknet_py/utils/merkle_tree.py
/// NOTE: Only supports the Pedersen hash for now.
#[derive(Debug, Clone)]
pub struct MerkleTree {
    leaves: Vec<FieldElement>,
    root_hash: FieldElement,
    levels: Vec<Vec<FieldElement>>,
}

/// The merkle proof that a leaf belongs to a Merkle tree.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MerkleProof(pub Vec<FieldElement>);

impl MerkleTree {
    pub fn new(leaves: Vec<FieldElement>) -> Result<Self, MerkleTreeError> {
        if leaves.is_empty() {
            return Err(MerkleTreeError::EmptyLeaves);
        }

        let mut tree = MerkleTree {
            leaves,
            root_hash: FieldElement::default(),
            levels: Vec::new(),
        };

        let (root_hash, levels) = tree.build();
        tree.root_hash = root_hash;
        tree.levels = levels;

        Ok(tree)
    }

    fn build(&self) -> (FieldElement, Vec<Vec<FieldElement>>) {
        if self.leaves.len() == 1 {
            return (self.leaves[0], vec![self.leaves.clone()]);
        }

        let mut curr_level_nodes = self.leaves.clone();
        let mut levels = Vec::new();

        while curr_level_nodes.len() > 1 {
            if curr_level_nodes.len() != self.leaves.len() {
                levels.push(curr_level_nodes.clone());
            }

            let mut new_nodes = Vec::new();
            for chunk in curr_level_nodes.chunks(2) {
                let a = chunk[0];
                let b = if chunk.len() > 1 {
                    chunk[1]
                } else {
                    FieldElement::ZERO
                };
                new_nodes.push(self.hash(&a, &b));
            }

            curr_level_nodes = new_nodes;
        }

        levels.insert(0, self.leaves.clone());
        levels.push(curr_level_nodes.clone());

        (curr_level_nodes[0], levels)
    }

    fn hash(&self, a: &FieldElement, b: &FieldElement) -> FieldElement {
        let (a_sorted, b_sorted) = match a.cmp(b) {
            Ordering::Less | Ordering::Equal => (a, b),
            Ordering::Greater => (b, a),
        };
        pedersen_hash(a_sorted, b_sorted)
    }

    pub fn get_proof(&self, leaf: &FieldElement) -> Option<MerkleProof> {
        let mut path = Vec::new();
        let mut current_hash = *leaf;

        for level in &self.levels {
            let index = level.iter().position(|&x| x == current_hash)?;
            if level.len() == 1 {
                break;
            }

            let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };
            let sibling = level.get(sibling_index).unwrap_or(&FieldElement::ZERO);

            path.push(*sibling);
            current_hash = self.hash(&current_hash, sibling);
        }
        Some(MerkleProof(path))
    }

    pub fn verify_proof(&self, leaf: &FieldElement, proof: &MerkleProof) -> bool {
        let mut current_hash = *leaf;
        for &sibling in &proof.0 {
            current_hash = self.hash(&current_hash, &sibling);
        }
        current_hash == self.root_hash
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_merkle_tree_new() {
        let leaves = vec![
            FieldElement::from(1_u32),
            FieldElement::from(2_u32),
            FieldElement::from(3_u32),
            FieldElement::from(4_u32),
        ];

        let merkle_tree = MerkleTree::new(leaves.clone()).unwrap();

        assert_eq!(merkle_tree.leaves, leaves);
        assert_eq!(merkle_tree.levels.len(), 3);
        assert_eq!(
            merkle_tree.root_hash,
            FieldElement::from_hex_be(
                "0x38118a340bbba28e678413cd3b07a9436a5e60fd6a7cbda7db958a6d501e274"
            )
            .unwrap()
        )
    }

    #[rstest]
    fn test_merkle_tree_proof() {
        let leaves = vec![
            FieldElement::from(1_u32),
            FieldElement::from(2_u32),
            FieldElement::from(3_u32),
            FieldElement::from(4_u32),
        ];
        let merkle_tree = MerkleTree::new(leaves.clone()).unwrap();

        let leaf = FieldElement::from(1_u32);
        let proof = merkle_tree.get_proof(&leaf).unwrap();

        let expected_proof = MerkleProof(vec![
            FieldElement::from_hex_be("0x2").unwrap(),
            FieldElement::from_hex_be(
                "0x262697b88544f733e5c6907c3e1763131e9f14c51ee7951258abbfb29415fbf",
            )
            .unwrap(),
        ]);

        assert_eq!(proof, expected_proof);
        assert!(merkle_tree.verify_proof(&leaf, &proof));
    }
}
