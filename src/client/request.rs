use std::time::Duration;

use rand::Rng;
use serde_json::Value;

use super::NotionClient;
use crate::error::{CliError, ErrorResponse};

impl NotionClient {
    fn api_url(&self, path: &str) -> Result<url::Url, CliError> {
        if url::Url::parse(path).is_ok() {
            return Err(CliError::Config(
                "API path must be relative to the configured Notion API origin".to_string(),
            ));
        }

        let url = self
            .base_url
            .join(path)
            .map_err(|e| CliError::Config(format!("Invalid API path {path}: {e}")))?;

        if url.origin() != self.base_url.origin() {
            return Err(CliError::Config(
                "API path must not change the configured Notion API origin".to_string(),
            ));
        }

        Ok(url)
    }

    /// Send a GET request to a Notion API endpoint.
    pub async fn get(&self, path: &str) -> Result<Value, CliError> {
        let url = self.api_url(path)?;

        self.request_with_retry(|| self.http.get(url.clone()).headers(self.notion_headers()))
            .await
    }

    /// Send a POST request with a JSON body.
    pub async fn post(&self, path: &str, body: &Value) -> Result<Value, CliError> {
        let url = self.api_url(path)?;

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
        let url = self.api_url(path)?;

        self.request_with_retry(|| {
            self.http
                .patch(url.clone())
                .headers(self.notion_headers())
                .json(body)
        })
        .await
    }

    /// Send a PUT request with a JSON body.
    #[allow(dead_code)]
    pub async fn put(&self, path: &str, body: &Value) -> Result<Value, CliError> {
        let url = self.api_url(path)?;

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
        let url = self.api_url(path)?;

        self.request_with_retry(|| self.http.delete(url.clone()).headers(self.notion_headers()))
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
                    rand::thread_rng().gen_range(0..=jitter_range * 2) as i64 - jitter_range as i64
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

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use url::Url;
    use wiremock::{
        matchers::{any, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    const TEST_TOKEN: &str = "secret_test_token";
    const TEST_API_VERSION: &str = "2026-03-11";

    #[derive(Clone, Copy, Debug)]
    enum RequestMethod {
        Get,
        Post,
        Patch,
        Put,
        Delete,
    }

    impl RequestMethod {
        const ALL: [Self; 5] = [Self::Get, Self::Post, Self::Patch, Self::Put, Self::Delete];

        async fn send(self, client: &NotionClient, path: &str) -> Result<Value, CliError> {
            let body = json!({ "test": true });
            match self {
                Self::Get => client.get(path).await,
                Self::Post => client.post(path, &body).await,
                Self::Patch => client.patch(path, &body).await,
                Self::Put => client.put(path, &body).await,
                Self::Delete => client.delete(path).await,
            }
        }
    }

    fn client_for(server: &MockServer) -> NotionClient {
        let base_url = Url::parse(&format!("{}/", server.uri())).unwrap();
        NotionClient::new(TEST_TOKEN.to_string())
            .unwrap()
            .with_base_url(base_url)
    }

    async fn mount_json_response(server: &MockServer, request_path: &str) {
        Mock::given(any())
            .and(path(request_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
            .mount(server)
            .await;
    }

    async fn assert_all_methods_reject_cross_origin_path(path: &str, attacker: &MockServer) {
        let notion = MockServer::start().await;
        let client = client_for(&notion);

        for method in RequestMethod::ALL {
            let result = method.send(&client, path).await;

            assert!(
                matches!(result, Err(CliError::Config(_))),
                "{method:?} must reject a cross-origin path with CliError::Config, got {result:?}"
            );
        }

        assert!(
            attacker.received_requests().await.unwrap().is_empty(),
            "rejected paths must not reach the attacker listener"
        );
    }

    #[tokio::test]
    async fn notion_api_requests_keep_authentication_headers() {
        let notion = MockServer::start().await;
        mount_json_response(&notion, "/v1/test").await;
        let client = client_for(&notion);

        for method in RequestMethod::ALL {
            method.send(&client, "/v1/test").await.unwrap();
        }

        let requests = notion.received_requests().await.unwrap();
        assert_eq!(requests.len(), RequestMethod::ALL.len());
        for request in requests {
            assert_eq!(
                request.headers.get("authorization").unwrap(),
                &format!("Bearer {TEST_TOKEN}")
            );
            assert_eq!(
                request.headers.get("notion-version").unwrap(),
                TEST_API_VERSION
            );
        }
    }

    #[tokio::test]
    async fn all_request_methods_reject_absolute_urls_before_sending() {
        let attacker = MockServer::start().await;
        mount_json_response(&attacker, "/capture").await;
        let path = format!("{}/capture", attacker.uri());

        assert_all_methods_reject_cross_origin_path(&path, &attacker).await;
    }

    #[tokio::test]
    async fn all_request_methods_reject_protocol_relative_urls_before_sending() {
        let attacker = MockServer::start().await;
        mount_json_response(&attacker, "/capture").await;
        let path = format!("//{}/capture", attacker.address());

        assert_all_methods_reject_cross_origin_path(&path, &attacker).await;
    }

    #[tokio::test]
    async fn all_request_methods_reject_backslash_authority_urls_before_sending() {
        let attacker = MockServer::start().await;
        mount_json_response(&attacker, "/capture").await;
        let path = format!(r"\\{}/capture", attacker.address());

        assert_all_methods_reject_cross_origin_path(&path, &attacker).await;
    }
}
