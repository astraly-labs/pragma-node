pub mod starkex;
pub mod typed_data;

use pragma_common::types::ConversionError;
use starknet::{
    core::{crypto::EcdsaSignError, types::FieldElement},
    signers::SigningKey,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SigningError {
    #[error("cannot convert type")]
    ConversionError,
    #[error("cannot sign: {0}")]
    SigningError(EcdsaSignError),
}

pub trait Signable {
    fn try_get_hash(&self) -> Result<FieldElement, ConversionError>;
}

/// Sign the passed data with the signer & return the signature 0x prefixed.
pub fn sign_data(signer: &SigningKey, data: &impl Signable) -> Result<String, SigningError> {
    let hash_to_sign = data
        .try_get_hash()
        .map_err(|_| SigningError::ConversionError)?;
    let signature = signer
        .sign(&hash_to_sign)
        .map_err(SigningError::SigningError)?;
    Ok(format!("0x{:}", signature))
}
