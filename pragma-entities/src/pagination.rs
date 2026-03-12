use serde::{Deserialize, Deserializer};
use utoipa::{IntoParams, ToSchema};

const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PAGE_SIZE: i64 = 1000;

/// Common pagination parameters that can be used across different endpoints
#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct PaginationParams {
    /// Page number (1-based). Defaults to 1
    #[serde(
        default = "default_page",
        deserialize_with = "deserialize_number_from_string"
    )]
    pub page: i64,
    /// Number of items per page. Defaults to 1000, max 1000
    #[serde(
        default = "default_page_size",
        deserialize_with = "deserialize_number_from_string"
    )]
    pub page_size: i64,
}

fn default_page() -> i64 {
    DEFAULT_PAGE
}

fn default_page_size() -> i64 {
    DEFAULT_PAGE_SIZE
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: default_page(),
            page_size: default_page_size(),
        }
    }
}

impl PaginationParams {
    pub const MAX_PAGE_SIZE: i64 = 1000;

    /// Gets validated `page`
    pub fn page(&self) -> i64 {
        if self.page <= 0 {
            DEFAULT_PAGE
        } else {
            self.page
        }
    }

    /// Gets validated `page_size`
    pub fn page_size(&self) -> i64 {
        self.page_size.clamp(1, Self::MAX_PAGE_SIZE)
    }

    /// Calculates the offset for database queries
    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.page_size()
    }

    /// Gets the limit for database queries (`page_size` + 1 to check for next page)
    pub fn limit_with_lookahead(&self) -> i64 {
        self.page_size() + 1
    }
}

/// Common pagination response fields that can be included in API responses
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PaginationResponse {
    pub page: i64,
    pub page_size: i64,
    pub has_next_page: bool,
}

impl PaginationResponse {
    pub fn new(page: i64, page_size: i64, has_next_page: bool) -> Self {
        Self {
            page,
            page_size,
            has_next_page,
        }
    }
}

fn deserialize_number_from_string<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct NumberFromStringVisitor;

    impl Visitor<'_> for NumberFromStringVisitor {
        type Value = i64;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a string or integer")
        }

        fn visit_str<E>(self, value: &str) -> Result<i64, E>
        where
            E: de::Error,
        {
            value.parse().map_err(de::Error::custom)
        }

        fn visit_i64<E>(self, value: i64) -> Result<i64, E>
        where
            E: de::Error,
        {
            Ok(value)
        }
    }

    deserializer.deserialize_any(NumberFromStringVisitor)
}
