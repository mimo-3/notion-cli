pub mod auth;
pub mod pagination;
pub mod request;

use url::Url;

use crate::cli::GlobalOpts;
use crate::config::Config;
use crate::error::CliError;

const DEFAULT_BASE_URL: &str = "https://api.notion.com";
const DEFAULT_API_VERSION: &str = "2026-03-11";

pub struct NotionClient {
    pub(crate) http: reqwest::Client,
    pub(crate) base_url: Url,
    pub(crate) token: String,
    pub(crate) api_version: String,
    pub(crate) max_retries: u32,
    pub(crate) dry_run: bool,
}

impl NotionClient {
    /// Build a NotionClient from CLI global options and config.
    pub fn from_opts(opts: &GlobalOpts, config: &Config) -> Result<Self, CliError> {
        let token = config.resolve_token(opts.token.as_deref(), opts.profile.as_deref())?;
        let api_version = opts
            .api_version
            .clone()
            .unwrap_or_else(|| DEFAULT_API_VERSION.to_string());

        let base_url = Url::parse(DEFAULT_BASE_URL).expect("default base URL should always parse");

        let http = reqwest::Client::builder()
            .user_agent(format!("notion-cli/{}", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        Ok(Self {
            http,
            base_url,
            token,
            api_version,
            max_retries: 3,
            dry_run: opts.dry_run,
        })
    }

    /// Build a NotionClient directly from a token (for testing or simple use).
    pub fn new(token: String) -> Result<Self, CliError> {
        let base_url = Url::parse(DEFAULT_BASE_URL).expect("default base URL should always parse");
        let http = reqwest::Client::builder()
            .user_agent(format!("notion-cli/{}", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        Ok(Self {
            http,
            base_url,
            token,
            api_version: DEFAULT_API_VERSION.to_string(),
            max_retries: 3,
            dry_run: false,
        })
    }

    #[cfg(test)]
    pub fn with_base_url(mut self, url: Url) -> Self {
        self.base_url = url;
        self
    }
}
