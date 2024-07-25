pub mod starkex;
pub mod typed_data;

use pragma_common::types::ConversionError;
use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};
use starknet::{
    core::{
        crypto::{ecdsa_verify, EcdsaSignError, Signature},
        types::FieldElement,
    },
    signers::SigningKey,
};
use thiserror::Error;

use crate::{
    handlers::entries::SignedRequest,
    types::entries::{build_publish_message, EntryTrait},
};

#[derive(Debug, Error)]
pub enum SigningError {
    #[error("cannot convert type")]
    ConversionError,
    #[error("cannot sign: {0}")]
    SigningError(#[from] EcdsaSignError),
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

/// Assert that a Signature is correct
pub fn assert_signature_is_valid<T>(
    request: &T,
    account_address: &FieldElement,
    public_key: &FieldElement,
    is_legacy: Option<bool>,
) -> Result<Signature, EntryError>
where
    T: SignedRequest,
    <T as SignedRequest>::EntryType: EntryTrait,
    for<'de> <T as SignedRequest>::EntryType: EntryTrait + Serialize + Deserialize<'de>,
{
    let published_message = build_publish_message(request.entries(), is_legacy)?;
    let message_hash = published_message.message_hash(*account_address);

    let signature_request = request.signature();
    let signature = Signature {
        r: signature_request[0],
        s: signature_request[1],
    };
    if !ecdsa_verify(public_key, &message_hash, &signature).map_err(EntryError::InvalidSignature)? {
        return Err(EntryError::Unauthorized);
    }
    Ok(signature)
}
