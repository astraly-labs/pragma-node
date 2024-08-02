use std::cmp::Ordering;

use color_eyre::eyre::eyre;
use color_eyre::eyre::Result;
use starknet_crypto::pedersen_hash;
use starknet_crypto::Felt;

/// Simple MerkleTree.
/// Reference:
/// https://github.com/software-mansion/starknet.py/blob/development/starknet_py/utils/merkle_tree.py
/// NOTE: Only supports the Pedersen hash for now.
#[derive(Debug, Clone)]
pub struct MerkleTree {
    leaves: Vec<Felt>,
    root_hash: Felt,
    levels: Vec<Vec<Felt>>,
}

/// The merkle proof that a leaf belongs to a Merkle tree.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MerkleProof(pub Vec<Felt>);

impl MerkleTree {
    pub fn new(leaves: Vec<Felt>) -> Result<Self> {
        if leaves.is_empty() {
            return Err(eyre!(
                "Cannot build Merkle tree from an empty list of leaves."
            ));
        }

        let mut tree = MerkleTree {
            leaves,
            root_hash: Felt::default(),
            levels: Vec::new(),
        };

        let (root_hash, levels) = tree.build();
        tree.root_hash = root_hash;
        tree.levels = levels;

        Ok(tree)
    }

    fn build(&self) -> (Felt, Vec<Vec<Felt>>) {
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
                    Felt::ZERO
                };
                new_nodes.push(self.hash(&a, &b));
            }

            curr_level_nodes = new_nodes;
        }

        levels.insert(0, self.leaves.clone());
        levels.push(curr_level_nodes.clone());

        (curr_level_nodes[0], levels)
    }

    fn hash(&self, a: &Felt, b: &Felt) -> Felt {
        let (a_sorted, b_sorted) = match a.cmp(b) {
            Ordering::Less | Ordering::Equal => (a, b),
            Ordering::Greater => (b, a),
        };
        pedersen_hash(a_sorted, b_sorted)
    }

    pub fn get_proof(&self, leaf: &Felt) -> Option<MerkleProof> {
        let mut path = Vec::new();
        let mut current_hash = *leaf;

        for level in &self.levels {
            let index = level.iter().position(|&x| x == current_hash)?;
            if level.len() == 1 {
                break;
            }

            let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };
            let sibling = level.get(sibling_index).unwrap_or(&Felt::ZERO);

            path.push(*sibling);
            current_hash = self.hash(&current_hash, sibling);
        }
        Some(MerkleProof(path))
    }

    pub fn verify_proof(&self, leaf: &Felt, proof: &MerkleProof) -> bool {
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
        let leaves = vec![Felt::from(1), Felt::from(2), Felt::from(3), Felt::from(4)];

        let merkle_tree = MerkleTree::new(leaves.clone()).unwrap();

        assert_eq!(merkle_tree.leaves, leaves);
        assert_eq!(merkle_tree.levels.len(), 3);
        assert_eq!(
            merkle_tree.root_hash,
            Felt::from_hex_unchecked(
                "0x38118a340bbba28e678413cd3b07a9436a5e60fd6a7cbda7db958a6d501e274"
            )
        )
    }

    #[rstest]
    fn test_merkle_tree_proof() {
        let leaves = vec![Felt::from(1), Felt::from(2), Felt::from(3), Felt::from(4)];
        let merkle_tree = MerkleTree::new(leaves.clone()).unwrap();

        let leaf = Felt::from(1);
        let proof = merkle_tree.get_proof(&leaf).unwrap();

        let expected_proof = MerkleProof(vec![
            Felt::from_hex_unchecked("0x2"),
            Felt::from_hex_unchecked(
                "0x262697b88544f733e5c6907c3e1763131e9f14c51ee7951258abbfb29415fbf",
            ),
        ]);

        assert_eq!(proof, expected_proof);
        assert!(merkle_tree.verify_proof(&leaf, &proof));
    }
}
