use aws_sdk_secretsmanager::Client;
use starknet::{core::types::FieldElement, signers::SigningKey};

const AWS_PRAGMA_PRIVATE_KEY_SECRET: &str = "pragma-secret-key";

#[derive(Debug)]
pub enum AwsError {
    NoSecretFound,
}

pub async fn build_pragma_signer_from_aws() -> SigningKey {
    let aws_client = get_aws_client().await;
    let pragma_secret_key = get_aws_secret(&aws_client, AWS_PRAGMA_PRIVATE_KEY_SECRET)
        .await
        .expect("can't get find pragma secret key");
    let pragma_secret_key =
        FieldElement::from_hex_be(&pragma_secret_key).expect("can't parse secret key");
    SigningKey::from_secret_scalar(pragma_secret_key)
}

/// Builds an aws client from environment variables:
/// - AWS_ACCESS_KEY_ID
/// - AWS_SECRET_ACCESS_KEY
/// See:
/// https://docs.aws.amazon.com/sdk-for-rust/latest/dg/credentials.html
async fn get_aws_client() -> Client {
    let aws_config = aws_config::load_from_env().await;
    aws_sdk_secretsmanager::Client::new(&aws_config)
}

async fn get_aws_secret(client: &Client, secret_name: &str) -> Result<String, AwsError> {
    let response = client
        .get_secret_value()
        .secret_id(secret_name)
        .send()
        .await;
    match response {
        Ok(wrapped_secret) => {
            let secret = wrapped_secret
                .secret_string
                .expect("No secret string found");
            Ok(secret)
        }
        Err(_) => Err(AwsError::NoSecretFound),
    }
}
