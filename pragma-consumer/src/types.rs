pub use pragma_common::types::block_id::{BlockId, BlockTag};
/// Re-export of some types from our common library so they're publicly accessible
/// through the SDK.
pub use pragma_common::types::merkle_tree::MerkleProof;
pub use pragma_common::types::options::{
    Instrument, InstrumentError, OptionCurrency, OptionData, OptionType,
};

use pragma_common::{types::merkle_tree::FeltMerkleProof, utils::field_element_as_hex_string};
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

    pub fn as_hex_calldata(&self) -> Result<Vec<String>, CalldataError> {
        Ok(self
            .as_calldata()?
            .into_iter()
            .map(|f| field_element_as_hex_string(&f))
            .collect())
    }
}
