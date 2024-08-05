/// Required fields to connect to our PragmAPI.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    // TODO: base_url should be an Enum PROD, DEV, CUSTOM
    pub base_url: String,
    pub api_key: String,
}
