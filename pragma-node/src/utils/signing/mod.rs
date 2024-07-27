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

use crate::types::entries::{build_publish_message, EntryTrait};

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

/// Assert that a new entries request is correctly signed
/// by the publisher.
/// If it is, we return the signature.
pub fn assert_request_signature_is_valid<R, E>(
    new_entries_request: &R,
    publisher_account: &FieldElement,
    publisher_public_key: &FieldElement,
) -> Result<Signature, EntryError>
where
    R: AsRef<[FieldElement]> + AsRef<[E]>,
    E: EntryTrait + Serialize + for<'de> Deserialize<'de>,
{
    // We recently updated our Pragma-SDK. This included a breaking change for how we
    // sign the entries before publishing them.
    // We want to support our publishers who are still on the older version and
    // encourage them to upgrade before removing this legacy code. Until then,
    // we support both methods.
    // TODO: Remove this legacy handling while every publishers are on the 2.0 version.
    let signature = match assert_signature_is_valid::<R, E>(
        new_entries_request,
        publisher_account,
        publisher_public_key,
    ) {
        Ok(signature) => signature,
        Err(_) => {
            tracing::debug!(
                "assert_signature_is_valid failed. Trying again with legacy signature..."
            );
            assert_legacy_signature_is_valid::<R, E>(
                new_entries_request,
                publisher_account,
                publisher_public_key,
            )?
        }
    };
    Ok(signature)
}

/// Assert that a request (passed with the request for creating new
/// entries) is correctly signed by the publisher and in a valid format.
/// Returns the signature if it is correct.
fn assert_signature_is_valid<R, E>(
    new_entries_request: &R,
    account_address: &FieldElement,
    public_key: &FieldElement,
) -> Result<Signature, EntryError>
where
    R: AsRef<[FieldElement]> + AsRef<[E]>,
    E: EntryTrait + Serialize + for<'de> Deserialize<'de>,
{
    let entries: &[E] = new_entries_request.as_ref();
    let published_message = build_publish_message(entries, None)?;
    let message_hash = published_message.message_hash(*account_address);

    let signature_slice: &[FieldElement] = new_entries_request.as_ref();
    let signature = Signature {
        r: signature_slice[0],
        s: signature_slice[1],
    };

    if !ecdsa_verify(public_key, &message_hash, &signature).map_err(EntryError::InvalidSignature)? {
        tracing::error!("Invalid signature for message hash {:?}", &message_hash);
        return Err(EntryError::Unauthorized);
    }
    Ok(signature)
}

/// Assert that a legacy request (passed with the request for creating new
/// entries) is correctly signed by the publisher and in a valid format.
/// Returns the signature if it is correct.
/// NOTE: Used for legacy signatures that uses our SDK before version 2.0.
/// TODO: Remove this function when we stop supporting the old format
fn assert_legacy_signature_is_valid<R, E>(
    new_entries_request: &R,
    account_address: &FieldElement,
    public_key: &FieldElement,
) -> Result<Signature, EntryError>
where
    R: AsRef<[FieldElement]> + AsRef<[E]>,
    E: EntryTrait + Serialize + for<'de> Deserialize<'de>,
{
    let entries: &[E] = new_entries_request.as_ref();
    let published_message = build_publish_message(entries, None)?;
    let message_hash = published_message.message_hash(*account_address);

    let signature_slice: &[FieldElement] = new_entries_request.as_ref();
    let signature = Signature {
        r: signature_slice[0],
        s: signature_slice[1],
    };

    if !ecdsa_verify(public_key, &message_hash, &signature).map_err(EntryError::InvalidSignature)? {
        tracing::error!("Invalid signature for message hash {:?}", &message_hash);
        return Err(EntryError::Unauthorized);
    }
    Ok(signature)
}
