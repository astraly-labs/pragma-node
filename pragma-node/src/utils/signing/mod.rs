pub mod starkex;
pub mod typed_data;

use starknet::{
    core::{
        crypto::{EcdsaSignError, Signature},
        types::FieldElement,
    },
    signers::SigningKey,
};

/// Sign the passed data with the signer & return the signature 0x prefixed.
pub fn sign_data(signer: &SigningKey, data: FieldElement) -> Result<Signature, EcdsaSignError> {
    signer.sign(&data)
}
