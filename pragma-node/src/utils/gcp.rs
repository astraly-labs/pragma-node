use google_secretmanager1::hyper_rustls::HttpsConnector;
use google_secretmanager1::{SecretManager, hyper_rustls, hyper_util, yup_oauth2};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use starknet::{core::types::Felt, signers::SigningKey};

const GCP_PRAGMA_PRIVATE_KEY_SECRET: &str = "pragma-secret-key";
const GCP_JSON_STARK_PRIVATE_KEY_FIELD: &str = "STARK_PRIVATE_KEY";

pub type GcpManager = SecretManager<HttpsConnector<HttpConnector>>;

#[derive(Debug)]
pub enum GcpError {
    NoSecretFound,
    DeserializationError,
    ConnectionError(String),
}

pub struct PragmaSignerBuilder {
    is_production: bool,
}

impl PragmaSignerBuilder {
    pub const fn new() -> Self {
        Self {
            is_production: false,
        }
    }

    #[must_use]
    pub const fn production_mode(mut self) -> Self {
        self.is_production = true;
        self
    }

    #[must_use]
    pub const fn non_production_mode(mut self) -> Self {
        self.is_production = false;
        self
    }

    pub async fn build(self) -> Option<SigningKey> {
        if self.is_production {
            build_pragma_signer_from_gcp().await
        } else {
            Some(SigningKey::from_random())
        }
    }
}

impl Default for PragmaSignerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn build_pragma_signer_from_gcp() -> Option<SigningKey> {
    let gcp_client = get_gcp_client().await.ok()?;
    let secret_json_response = get_gcp_secret(&gcp_client, GCP_PRAGMA_PRIVATE_KEY_SECRET)
        .await
        .ok()?;
    let pragma_secret_key: String = get_pragma_secret_key(secret_json_response).ok()?;
    let pragma_secret_key = Felt::from_hex(&pragma_secret_key).ok()?;
    Some(SigningKey::from_secret_scalar(pragma_secret_key))
}

async fn get_gcp_client() -> Result<GcpManager, GcpError> {
    // Check if service account credentials are provided
    let auth = if let Ok(service_account_json) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        // Use service account credentials from file
        let service_account = yup_oauth2::ServiceAccountAuthenticator::builder(
            yup_oauth2::read_service_account_key(service_account_json)
                .await
                .map_err(|e| {
                    GcpError::ConnectionError(format!("Failed to read service account: {}", e))
                })?,
        )
        .build()
        .await
        .map_err(|e| {
            GcpError::ConnectionError(format!(
                "Failed to create service account authenticator: {}",
                e
            ))
        })?;

        service_account
    } else {
        // Fall back to application default credentials
        yup_oauth2::InstalledFlowAuthenticator::builder(
            yup_oauth2::ApplicationSecret::default(),
            yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
        )
        .build()
        .await
        .map_err(|e| GcpError::ConnectionError(format!("Failed to create authenticator: {}", e)))?
    };

    // Create a properly configured connector
    let https_connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|e| GcpError::ConnectionError(e.to_string()))?
        .https_or_http()
        .enable_http1()
        .build();

    // Create a client with the correct type
    let client =
        google_secretmanager1::hyper_util::client::legacy::Client::builder(TokioExecutor::new())
            .build(https_connector);
    // Create the SecretManager with the client and authenticator
    Ok(SecretManager::new(client, auth))
}

async fn get_gcp_secret(client: &GcpManager, secret_name: &str) -> Result<String, GcpError> {
    let project_id = std::env::var("GCP_PROJECT_ID").map_err(|_| GcpError::NoSecretFound)?;
    let secret_path = format!("projects/{project_id}/secrets/{secret_name}/versions/latest");

    let result = client
        .projects()
        .secrets_versions_access(&secret_path)
        .doit()
        .await
        .map_err(|_| GcpError::NoSecretFound)?;

    // Get the payload from the result
    let payload = result.1.payload.ok_or(GcpError::NoSecretFound)?;

    // Get the data from the payload
    let data = payload.data.ok_or(GcpError::NoSecretFound)?;

    // Convert the data to a string
    String::from_utf8(data).map_err(|_| GcpError::DeserializationError)
}

fn get_pragma_secret_key(secret_json_response: String) -> Result<String, GcpError> {
    let secret_json: serde_json::Value =
        serde_json::from_str(&secret_json_response).map_err(|_| GcpError::DeserializationError)?;

    let pragma_secret_key = secret_json
        .get(GCP_JSON_STARK_PRIVATE_KEY_FIELD)
        .ok_or(GcpError::DeserializationError)?
        .as_str()
        .ok_or(GcpError::DeserializationError)?;

    Ok(pragma_secret_key.to_string())
}
