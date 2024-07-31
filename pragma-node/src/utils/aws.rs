use aws_sdk_secretsmanager::Client;
use starknet::{core::types::Felt, signers::SigningKey};

const AWS_PRAGMA_PRIVATE_KEY_SECRET: &str = "pragma-secret-key";
const AWS_JSON_STARK_PRIVATE_KEY_FIELD: &str = "STARK_PRIVATE_KEY";

#[derive(Debug)]
pub enum AwsError {
    NoSecretFound,
    DeserializationError,
}

pub struct PragmaSignerBuilder {
    is_production: bool,
}

impl PragmaSignerBuilder {
    pub fn new() -> Self {
        Self {
            is_production: false,
        }
    }

    pub fn production_mode(mut self) -> Self {
        self.is_production = true;
        self
    }

    pub fn non_production_mode(mut self) -> Self {
        self.is_production = false;
        self
    }

    pub async fn build(self) -> Option<SigningKey> {
        if self.is_production {
            build_pragma_signer_from_aws().await
        } else {
            Some(SigningKey::from_random())
        }
    }
}

pub async fn build_pragma_signer_from_aws() -> Option<SigningKey> {
    let aws_client = get_aws_client().await;
    let secret_json_response = get_aws_secret(&aws_client, AWS_PRAGMA_PRIVATE_KEY_SECRET)
        .await
        .ok()?;
    let pragma_secret_key: String = get_pragma_secret_key(secret_json_response).ok()?;
    let pragma_secret_key = Felt::from_hex(&pragma_secret_key).ok()?;
    Some(SigningKey::from_secret_scalar(pragma_secret_key))
}

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
                .ok_or(AwsError::NoSecretFound)?;
            Ok(secret)
        }
        Err(_) => Err(AwsError::NoSecretFound),
    }
}

fn get_pragma_secret_key(secret_json_response: String) -> Result<String, AwsError> {
    let secret_json: serde_json::Value =
        serde_json::from_str(&secret_json_response).map_err(|_| AwsError::DeserializationError)?;
    let pragma_secret_key = secret_json[AWS_JSON_STARK_PRIVATE_KEY_FIELD]
        .as_str()
        .ok_or(AwsError::DeserializationError)?;
    Ok(pragma_secret_key.to_string())
}
