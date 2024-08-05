use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;
use thiserror::Error;

use crate::hash::pedersen_hash;

#[derive(Debug, Error)]
pub enum MerkleTreeError {
    #[error("could not build the merkle tree: {0}")]
    BuildFailed(String),
    #[error("cannot build a merkle tree from empty leaves")]
    EmptyLeaves,
    #[error("could not convert hash {0} to a felt")]
    FeltConversion(String),
}

/// Simple MerkleTree.
/// Reference:
/// https://github.com/software-mansion/starknet.py/blob/development/starknet_py/utils/merkle_tree.py
/// NOTE: Only supports the Pedersen hash for now.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerkleTree {
    pub root_hash: FieldElement,
    pub leaves: Vec<FieldElement>,
    pub levels: Vec<Vec<FieldElement>>,
}

/// The merkle proof that a leaf belongs to a Merkle tree.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FeltMerkleProof(pub Vec<FieldElement>);

/// The merkle proof but with hexadecimal hashes instead of Field elements.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MerkleProof(pub Vec<String>);

impl From<FeltMerkleProof> for MerkleProof {
    fn from(proof: FeltMerkleProof) -> Self {
        MerkleProof(
            proof
                .0
                .clone()
                .into_iter()
                .map(|felt| format!("{:#x}", felt))
                .collect(),
        )
    }
}

impl TryInto<FeltMerkleProof> for MerkleProof {
    type Error = MerkleTreeError;

    fn try_into(self) -> Result<FeltMerkleProof, Self::Error> {
        self.0
            .into_iter()
            .map(|hash| {
                FieldElement::from_hex_be(&hash).map_err(|_| MerkleTreeError::FeltConversion(hash))
            })
            .collect::<Result<Vec<FieldElement>, _>>()
            .map(FeltMerkleProof)
    }
}

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
                // sorting of A & B happens in the [hash] method
                new_nodes.push(pedersen_hash(&a, &b));
            }

            curr_level_nodes = new_nodes;
        }

        levels.insert(0, self.leaves.clone());
        levels.push(curr_level_nodes.clone());

        (curr_level_nodes[0], levels)
    }

    /// Returns the merkle proof if the passed leaf is found in the tree.
    pub fn get_proof(&self, leaf: &FieldElement) -> Option<FeltMerkleProof> {
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
            current_hash = pedersen_hash(&current_hash, sibling);
        }
        Some(FeltMerkleProof(path))
    }

    /// Verify that the passed merkle proof is valid for the leaf.
    pub fn verify_proof(&self, leaf: &FieldElement, proof: &FeltMerkleProof) -> bool {
        let mut current_hash = *leaf;
        for &sibling in &proof.0 {
            current_hash = pedersen_hash(&current_hash, &sibling);
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

        let expected_proof = FeltMerkleProof(vec![
            FieldElement::from_hex_be("0x2").unwrap(),
            FieldElement::from_hex_be(
                "0x262697b88544f733e5c6907c3e1763131e9f14c51ee7951258abbfb29415fbf",
            )
            .unwrap(),
        ]);

        assert_eq!(proof, expected_proof);
        assert!(merkle_tree.verify_proof(&leaf, &proof));
    }

    #[rstest]
    fn test_merkle_tree_single_leaf() {
        let leaves = vec![FieldElement::from(1_u32)];
        let merkle_tree = MerkleTree::new(leaves.clone()).unwrap();

        assert_eq!(merkle_tree.leaves, leaves);
        assert_eq!(merkle_tree.levels.len(), 1);
        assert_eq!(merkle_tree.root_hash, FieldElement::from(1_u32));
    }

    #[rstest]
    fn test_merkle_tree_odd_number_of_leaves() {
        let leaves = vec![
            FieldElement::from(1_u32),
            FieldElement::from(2_u32),
            FieldElement::from(3_u32),
        ];
        let merkle_tree = MerkleTree::new(leaves.clone()).unwrap();

        assert_eq!(merkle_tree.leaves, leaves);
        assert_eq!(merkle_tree.levels.len(), 3);
        assert_eq!(
            merkle_tree.root_hash,
            FieldElement::from_hex_be(
                "0x015ac9e457789ef0c56e5d559809e7336a909c14ee2511503fa7af69be1ba639"
            )
            .unwrap()
        );
    }

    #[rstest]
    fn test_merkle_tree_empty_leaves() {
        let leaves: Vec<FieldElement> = vec![];
        let result = MerkleTree::new(leaves);

        assert!(matches!(result, Err(MerkleTreeError::EmptyLeaves)));
    }

    #[rstest]
    fn test_merkle_tree_proof_verification_failure() {
        let leaves = vec![
            FieldElement::from(1_u32),
            FieldElement::from(2_u32),
            FieldElement::from(3_u32),
            FieldElement::from(4_u32),
        ];
        let merkle_tree = MerkleTree::new(leaves.clone()).unwrap();

        let leaf = FieldElement::from(1_u32);
        let mut proof = merkle_tree.get_proof(&leaf).unwrap();

        if let Some(first) = proof.0.first_mut() {
            *first = FieldElement::from(99_u32);
        }

        assert!(!merkle_tree.verify_proof(&leaf, &proof));
    }

    #[rstest]
    fn test_merkle_tree_proof_for_nonexistent_leaf() {
        let leaves = vec![
            FieldElement::from(1_u32),
            FieldElement::from(2_u32),
            FieldElement::from(3_u32),
            FieldElement::from(4_u32),
        ];
        let merkle_tree = MerkleTree::new(leaves).unwrap();

        let nonexistent_leaf = FieldElement::from(5_u32);
        let proof = merkle_tree.get_proof(&nonexistent_leaf);

        assert!(proof.is_none());
    }
}
