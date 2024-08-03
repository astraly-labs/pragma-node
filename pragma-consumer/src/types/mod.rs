use pragma_common::types::{merkle_tree::MerkleProof, options::OptionData};

/// Calldata used to query Pragma Oracle.
#[derive(Debug, Default)]
pub struct MerkleFeedCalldata {
    pub merkle_proof: MerkleProof,
    pub option_data: OptionData,
}
