use std::time::Duration;

use rand::Rng;
use serde_json::Value;

use super::NotionClient;
use crate::error::{CliError, ErrorResponse};

impl NotionClient {
    /// Send a GET request to a Notion API endpoint.
    pub async fn get(&self, path: &str) -> Result<Value, CliError> {
        let url = self.base_url.join(path).map_err(|e| {
            CliError::Config(format!("Invalid API path {path}: {e}"))
        })?;

        self.request_with_retry(|| {
            self.http
                .get(url.clone())
                .headers(self.notion_headers())
        })
        .await
    }

    /// Send a POST request with a JSON body.
    pub async fn post(&self, path: &str, body: &Value) -> Result<Value, CliError> {
        let url = self.base_url.join(path).map_err(|e| {
            CliError::Config(format!("Invalid API path {path}: {e}"))
        })?;

        self.request_with_retry(|| {
            self.http
                .post(url.clone())
                .headers(self.notion_headers())
                .json(body)
        })
        .await
    }

    /// Send a PATCH request with a JSON body.
    pub async fn patch(&self, path: &str, body: &Value) -> Result<Value, CliError> {
        let url = self.base_url.join(path).map_err(|e| {
            CliError::Config(format!("Invalid API path {path}: {e}"))
        })?;

        self.request_with_retry(|| {
            self.http
                .patch(url.clone())
                .headers(self.notion_headers())
                .json(body)
        })
        .await
    }

    /// Send a PUT request with a JSON body.
    pub async fn put(&self, path: &str, body: &Value) -> Result<Value, CliError> {
        let url = self.base_url.join(path).map_err(|e| {
            CliError::Config(format!("Invalid API path {path}: {e}"))
        })?;

        self.request_with_retry(|| {
            self.http
                .put(url.clone())
                .headers(self.notion_headers())
                .json(body)
        })
        .await
    }

    /// Send a DELETE request.
    pub async fn delete(&self, path: &str) -> Result<Value, CliError> {
        let url = self.base_url.join(path).map_err(|e| {
            CliError::Config(format!("Invalid API path {path}: {e}"))
        })?;

        self.request_with_retry(|| {
            self.http
                .delete(url.clone())
                .headers(self.notion_headers())
        })
        .await
    }

    /// Execute a request with retry logic for 429/529 responses.
    async fn request_with_retry<F>(&self, build_request: F) -> Result<Value, CliError>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        let mut last_retry_after = 1u64;

        for attempt in 0..=self.max_retries {
            let response = build_request().send().await?;
            let status = response.status();

            if status.is_success() {
                let body: Value = response.json().await?;
                return Ok(body);
            }

            // Rate limited or overloaded
            if status.as_u16() == 429 || status.as_u16() == 529 {
                if attempt == self.max_retries {
                    return Err(CliError::RateLimited {
                        retry_after: last_retry_after,
                    });
                }

                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(1);

                // Exponential backoff with jitter
                let backoff = retry_after.saturating_mul(1 << attempt).min(60);
                let jitter_range = (backoff as f64 * 0.2) as u64;
                let jitter = if jitter_range > 0 {
                    rand::thread_rng().gen_range(0..=jitter_range * 2) as i64
                        - jitter_range as i64
                } else {
                    0
                };
                let wait = (backoff as i64 + jitter).max(1) as u64;
                last_retry_after = wait;

                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }

            // Other errors
            let error_body = response.text().await.unwrap_or_default();
            if let Ok(err_resp) = serde_json::from_str::<ErrorResponse>(&error_body) {
                return Err(CliError::Api {
                    status: err_resp.status,
                    code: err_resp.code,
                    message: err_resp.message,
                });
            }

            return Err(CliError::Api {
                status: status.as_u16(),
                code: "unknown".to_string(),
                message: error_body,
            });
        }

        Err(CliError::RateLimited {
            retry_after: last_retry_after,
        })
    }
}
