use pragma_common::types::{instrument::OptionData, merkle_tree::MerkleProof};

/// Calldata used to query Pragma Oracle.
#[derive(Debug, Default)]
pub struct MerkleFeedCalldata {
    pub merkle_proof: MerkleProof,
    pub option_data: OptionData,
}
