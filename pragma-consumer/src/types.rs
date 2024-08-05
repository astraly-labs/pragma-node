use pragma_common::types::{merkle_tree::HexaMerkleProof, options::OptionData};

/// Calldata used to query Pragma Oracle.
#[derive(Debug)]
pub struct MerkleFeedCalldata {
    pub merkle_proof: HexaMerkleProof,
    pub option_data: OptionData,
}
