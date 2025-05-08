use pragma_common::starknet::{ConversionError, SignerError};
use starknet::signers::SigningKey;
use starknet_crypto::Felt;

pub trait Signable {
    fn try_get_hash(&self) -> Result<Felt, ConversionError>;
}

/// Sign the passed data with the signer & return the signature 0x prefixed.
pub fn sign_data(signer: &SigningKey, data: &impl Signable) -> Result<String, SignerError> {
    let hash_to_sign = data.try_get_hash()?;
    let signature = signer.sign(&hash_to_sign)?;
    Ok(format!("0x{signature:}"))
}
