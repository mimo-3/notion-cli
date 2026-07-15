use std::fmt;

#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error("Not authenticated. Run `notion auth login` first.")]
    NotAuthenticated,

    #[error("API error ({status}): {code} - {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
    },

    #[error("Rate limited. Retry after {retry_after}s")]
    RateLimited { retry_after: u64 },

    #[error("Invalid filter: {0}")]
    #[allow(dead_code)]
    FilterParse(String),

    #[error("Invalid ID format: {0}")]
    #[allow(dead_code)]
    InvalidId(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("OAuth error: {0}")]
    OAuth(String),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::NotAuthenticated => 3,
            CliError::RateLimited { .. } => 4,
            _ => 1,
        }
    }
}

/// Notion API error response body.
#[derive(serde::Deserialize, Debug)]
pub struct ErrorResponse {
    pub status: u16,
    pub code: String,
    pub message: String,
}

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}): {}", self.code, self.status, self.message)
    }
}
