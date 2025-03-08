use google_secretmanager1::hyper_rustls::HttpsConnector;
use google_secretmanager1::{SecretManager, hyper_rustls, hyper_util, yup_oauth2};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use starknet::{core::types::Felt, signers::SigningKey};
use tracing::{debug, error, info};

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
    debug!("Starting to build pragma signer from GCP");
    let gcp_client = match get_gcp_client().await {
        Ok(client) => {
            debug!("Successfully created GCP client");
            client
        }
        Err(e) => {
            error!("Failed to create GCP client: {:?}", e);
            return None;
        }
    };

    let secret_json_response =
        match get_gcp_secret(&gcp_client, GCP_PRAGMA_PRIVATE_KEY_SECRET).await {
            Ok(secret) => {
                debug!("Successfully retrieved secret from GCP");
                secret
            }
            Err(e) => {
                error!("Failed to get GCP secret: {:?}", e);
                return None;
            }
        };

    let pragma_secret_key = match get_pragma_secret_key(secret_json_response) {
        Ok(key) => {
            debug!("Successfully extracted pragma secret key from JSON");
            key
        }
        Err(e) => {
            error!("Failed to extract pragma secret key: {:?}", e);
            return None;
        }
    };

    let pragma_secret_key = match Felt::from_hex(&pragma_secret_key) {
        Ok(felt) => {
            debug!("Successfully converted secret key to Felt");
            felt
        }
        Err(e) => {
            error!("Failed to convert secret key to Felt: {:?}", e);
            return None;
        }
    };

    info!("Successfully built pragma signer from GCP");
    Some(SigningKey::from_secret_scalar(pragma_secret_key))
}

async fn get_gcp_client() -> Result<GcpManager, GcpError> {
    debug!("Attempting to create GCP client");
    // Check if service account credentials are provided
    let auth = if let Ok(service_account_json) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        debug!(
            "Found GOOGLE_APPLICATION_CREDENTIALS environment variable: {}",
            service_account_json
        );
        // Use service account credentials from file
        let service_account_key = match yup_oauth2::read_service_account_key(&service_account_json)
            .await
        {
            Ok(key) => {
                debug!("Successfully read service account key file");
                key
            }
            Err(e) => {
                error!(
                    "Failed to read service account file at {}: {}",
                    service_account_json, e
                );
                error!(
                    "Service account file should contain: type, project_id, private_key_id, private_key, client_email, client_id, auth_uri, token_uri, auth_provider_x509_cert_url, client_x509_cert_url"
                );
                return Err(GcpError::ConnectionError(format!(
                    "Failed to read service account: {e}"
                )));
            }
        };

        match yup_oauth2::ServiceAccountAuthenticator::builder(service_account_key)
            .build()
            .await
        {
            Ok(authenticator) => {
                debug!("Successfully created service account authenticator");
                authenticator
            }
            Err(e) => {
                error!("Failed to create service account authenticator: {e}");
                return Err(GcpError::ConnectionError(format!(
                    "Failed to create service account authenticator: {e}"
                )));
            }
        }
    } else {
        debug!("No GOOGLE_APPLICATION_CREDENTIALS found, using application default credentials");
        // Fall back to application default credentials
        yup_oauth2::InstalledFlowAuthenticator::builder(
            yup_oauth2::ApplicationSecret::default(),
            yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
        )
        .build()
        .await
        .map_err(|e| {
            error!("Failed to create authenticator: {e}");
            GcpError::ConnectionError(format!("Failed to create authenticator: {e}"))
        })?
    };

    // Create a properly configured connector
    let https_connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|e| {
            error!("Failed to create HTTPS connector: {}", e);
            GcpError::ConnectionError(e.to_string())
        })?
        .https_or_http()
        .enable_http1()
        .build();

    // Create a client with the correct type
    let client =
        google_secretmanager1::hyper_util::client::legacy::Client::builder(TokioExecutor::new())
            .build(https_connector);
    // Create the SecretManager with the client and authenticator
    debug!("Successfully created GCP client");
    Ok(SecretManager::new(client, auth))
}

async fn get_gcp_secret(client: &GcpManager, secret_name: &str) -> Result<String, GcpError> {
    debug!("Attempting to retrieve secret: {}", secret_name);
    let project_id = std::env::var("GCP_PROJECT_ID").map_err(|_| {
        error!("GCP_PROJECT_ID environment variable not set");
        GcpError::NoSecretFound
    })?;
    let secret_path = format!("projects/{project_id}/secrets/{secret_name}/versions/latest");
    debug!("Secret path: {}", secret_path);

    let result = client
        .projects()
        .secrets_versions_access(&secret_path)
        .doit()
        .await
        .map_err(|e| {
            error!("Failed to access secret: {:?}", e);
            GcpError::NoSecretFound
        })?;

    // Get the payload from the result
    let payload = result.1.payload.ok_or_else(|| {
        error!("No payload found in secret response");
        GcpError::NoSecretFound
    })?;

    // Get the data from the payload
    let data = payload.data.ok_or_else(|| {
        error!("No data found in secret payload");
        GcpError::NoSecretFound
    })?;

    debug!("Successfully retrieved secret data");
    // Convert the data to a string
    String::from_utf8(data).map_err(|e| {
        error!("Failed to convert secret data to UTF-8: {}", e);
        GcpError::DeserializationError
    })
}

fn get_pragma_secret_key(secret_json_response: String) -> Result<String, GcpError> {
    debug!("Attempting to parse secret JSON response");
    let secret_json: serde_json::Value =
        serde_json::from_str(&secret_json_response).map_err(|e| {
            error!("Failed to parse secret JSON: {}", e);
            GcpError::DeserializationError
        })?;

    let pragma_secret_key = secret_json
        .get(GCP_JSON_STARK_PRIVATE_KEY_FIELD)
        .ok_or_else(|| {
            error!(
                "Field {} not found in secret JSON",
                GCP_JSON_STARK_PRIVATE_KEY_FIELD
            );
            GcpError::DeserializationError
        })?
        .as_str()
        .ok_or_else(|| {
            error!("Secret key is not a string");
            GcpError::DeserializationError
        })?;

    debug!("Successfully extracted secret key from JSON");
    Ok(pragma_secret_key.to_string())
}
