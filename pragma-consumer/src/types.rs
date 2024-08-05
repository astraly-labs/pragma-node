use pragma_common::types::{
    merkle_tree::{FeltMerkleProof, MerkleProof},
    options::OptionData,
};
use starknet::core::types::FieldElement;

#[derive(thiserror::Error, Debug)]
pub enum CalldataError {
    #[error("field element conversion failed")]
    FeltConversion,
}

/// Calldata used to query Pragma Oracle.
#[derive(Debug)]
pub struct MerkleFeedCalldata {
    pub merkle_proof: MerkleProof,
    pub option_data: OptionData,
}

impl MerkleFeedCalldata {
    /// Converts the structure as the Vec<FieldElement>, i.e. a calldata.
    pub fn as_calldata(&self) -> Result<Vec<FieldElement>, CalldataError> {
        let mut calldata = Vec::with_capacity(self.merkle_proof.0.len());

        let felt_proof: FeltMerkleProof = self
            .merkle_proof
            .clone()
            .try_into()
            .map_err(|_| CalldataError::FeltConversion)?;

        calldata.extend(felt_proof.0);

        let option_calldata = self
            .option_data
            .as_calldata()
            .map_err(|_| CalldataError::FeltConversion)?;
        calldata.extend(option_calldata);

        Ok(calldata)
    }
}
