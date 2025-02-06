pub mod starkex;

use crate::errors::ConversionError;
use serde::{Deserialize, Serialize};
use starknet::{
    core::{
        crypto::{ecdsa_verify, EcdsaSignError, Signature},
        types::Felt,
    },
    signers::SigningKey,
};
use thiserror::Error;
use utoipa::ToSchema;

use crate::types::entries::{build_publish_message, EntryTrait};
use crate::types::typed_data::TypedData;

#[derive(Debug, Error, ToSchema)]
pub enum SignerError {
    #[error(transparent)]
    ConversionError(#[from] ConversionError),
    #[error("cannot sign: {0}")]
    #[schema(value_type = String)]
    SigningError(#[from] EcdsaSignError),
    #[error("invalid signature for message hash {0:?}")]
    #[schema(value_type = String)]
    InvalidSignature(Felt),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("invalid message: {0}")]
    InvalidMessage(String),
}

pub trait Signable {
    fn try_get_hash(&self) -> Result<Felt, ConversionError>;
}

/// Sign the passed data with the signer & return the signature 0x prefixed.
pub fn sign_data(signer: &SigningKey, data: &impl Signable) -> Result<String, SignerError> {
    let hash_to_sign = data.try_get_hash()?;
    let signature = signer.sign(&hash_to_sign)?;
    Ok(format!("0x{:}", signature))
}

/// Assert that a new entries request is correctly signed
/// by the publisher.
/// If it is, we return the signature.
pub fn assert_request_signature_is_valid<R, E>(
    new_entries_request: &R,
    publisher_account: &Felt,
    publisher_public_key: &Felt,
) -> Result<Signature, SignerError>
where
    R: AsRef<[Felt]> + AsRef<[E]>,
    E: EntryTrait + Serialize + for<'de> Deserialize<'de>,
{
    let signature = assert_signature_is_valid::<R, E>(
        new_entries_request,
        publisher_account,
        publisher_public_key,
    )?;
    Ok(signature)
}

/// Assert that a request (passed with the request for creating new
/// entries) is correctly signed by the publisher and in a valid format.
/// Returns the signature if it is correct.
fn assert_signature_is_valid<R, E>(
    new_entries_request: &R,
    account_address: &Felt,
    public_key: &Felt,
) -> Result<Signature, SignerError>
where
    R: AsRef<[Felt]> + AsRef<[E]>,
    E: EntryTrait + Serialize + for<'de> Deserialize<'de>,
{
    let entries: &[E] = new_entries_request.as_ref();
    let published_message = build_publish_message(entries);
    let message_hash = published_message
        .encode(*account_address)
        .map_err(|e| SignerError::InvalidMessage(e.to_string()))?
        .hash;

    let signature_slice: &[Felt] = new_entries_request.as_ref();
    let signature = Signature {
        r: signature_slice[0],
        s: signature_slice[1],
    };

    if !ecdsa_verify(public_key, &message_hash, &signature)
        .map_err(|_| SignerError::InvalidSignature(message_hash))?
    {
        return Err(SignerError::Unauthorized(format!(
            "Invalid signature for message hash {:?}",
            &message_hash
        )));
    }
    Ok(signature)
}

pub fn assert_login_is_valid(
    login_message: TypedData,
    signature: &Signature,
    account_address: &Felt,
    public_key: &Felt,
) -> Result<(), SignerError> {
    let message_hash = login_message
        .encode(*account_address)
        .map_err(|e| SignerError::InvalidMessage(e.to_string()))?
        .hash;

    if !ecdsa_verify(public_key, &message_hash, signature)
        .map_err(|_| SignerError::InvalidSignature(message_hash))?
    {
        return Err(SignerError::Unauthorized(format!(
            "Invalid signature for message hash {:?}",
            &message_hash
        )));
    }
    Ok(())
}
