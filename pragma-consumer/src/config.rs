/// `PragmAPI` Base url. Can be either Dev, Prod or a Custom url.
#[derive(Debug, Clone)]
pub enum PragmaBaseUrl {
    Dev,
    Prod,
    Custom(String),
}

impl PragmaBaseUrl {
    pub fn url(&self) -> &str {
        match self {
            Self::Dev => "https://api.dev.pragma.build",
            Self::Prod => "https://api.prod.pragma.build",
            Self::Custom(url) => url,
        }
    }
}

/// Required fields to connect to our `PragmAPI`.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub base_url: PragmaBaseUrl,
    pub api_key: String,
}
