use reqwest::header::{HeaderMap, HeaderValue};

use super::NotionClient;

impl NotionClient {
    /// Build the standard Notion API headers.
    pub(crate) fn notion_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", self.token))
                .expect("token should be valid header value"),
        );
        headers.insert(
            "Notion-Version",
            HeaderValue::from_str(&self.api_version)
                .expect("api version should be valid header value"),
        );
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers
    }
}
