use serde_json::Value;

use super::NotionClient;
use crate::error::CliError;

/// Safety limit to prevent infinite pagination loops.
const MAX_PAGES: u32 = 10_000;

/// Options controlling pagination behavior.
#[derive(Debug, Clone)]
pub struct PaginationOpts {
    pub page_size: u8,
    pub start_cursor: Option<String>,
    pub fetch_all: bool,
    pub limit: Option<u32>,
}

impl Default for PaginationOpts {
    fn default() -> Self {
        Self {
            page_size: 50,
            start_cursor: None,
            fetch_all: false,
            limit: None,
        }
    }
}

impl NotionClient {
    /// Paginate through a POST endpoint (e.g. search, db query).
    /// Collects all results into a Vec.
    pub async fn paginate_post(
        &self,
        path: &str,
        base_body: &Value,
        opts: &PaginationOpts,
    ) -> Result<Vec<Value>, CliError> {
        let mut all_results = Vec::new();
        let mut cursor = opts.start_cursor.clone();
        let mut prev_cursor: Option<String> = None;
        let limit = opts.limit.unwrap_or(u32::MAX);
        let mut page_count: u32 = 0;

        loop {
            page_count += 1;
            if page_count > MAX_PAGES {
                return Err(CliError::Pagination(format!(
                    "Exceeded maximum page count ({MAX_PAGES})"
                )));
            }

            let mut body = base_body.clone();
            if let Some(obj) = body.as_object_mut() {
                obj.insert(
                    "page_size".to_string(),
                    Value::Number(opts.page_size.into()),
                );
                if let Some(ref c) = cursor {
                    obj.insert("start_cursor".to_string(), Value::String(c.clone()));
                }
            }

            let response = self.post(path, &body).await?;

            if let Some(results) = response.get("results").and_then(|v| v.as_array()) {
                for item in results {
                    if all_results.len() as u32 >= limit {
                        return Ok(all_results);
                    }
                    all_results.push(item.clone());
                }
            }

            if !opts.fetch_all && opts.limit.is_none() {
                // Only fetch first page
                break;
            }

            if all_results.len() as u32 >= limit {
                break;
            }

            let has_more = response
                .get("has_more")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !has_more {
                break;
            }

            cursor = response
                .get("next_cursor")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            match cursor {
                None => {
                    return Err(CliError::Pagination(
                        "Server indicated has_more=true but returned no next_cursor".into(),
                    ));
                }
                Some(ref new_c) if Some(new_c) == prev_cursor.as_ref() => {
                    return Err(CliError::Pagination(
                        "Server returned the same cursor twice".into(),
                    ));
                }
                _ => {}
            }

            prev_cursor = cursor.clone();
        }

        Ok(all_results)
    }

    /// Paginate through a GET endpoint with query params (e.g. block children).
    pub async fn paginate_get(
        &self,
        base_path: &str,
        opts: &PaginationOpts,
    ) -> Result<Vec<Value>, CliError> {
        let mut all_results = Vec::new();
        let mut cursor = opts.start_cursor.clone();
        let mut prev_cursor: Option<String> = None;
        let limit = opts.limit.unwrap_or(u32::MAX);
        let mut page_count: u32 = 0;

        loop {
            page_count += 1;
            if page_count > MAX_PAGES {
                return Err(CliError::Pagination(format!(
                    "Exceeded maximum page count ({MAX_PAGES})"
                )));
            }

            let sep = if base_path.contains('?') { '&' } else { '?' };
            let mut query = url::form_urlencoded::Serializer::new(String::new());
            query.append_pair("page_size", &opts.page_size.to_string());
            if let Some(ref c) = cursor {
                query.append_pair("start_cursor", c);
            }
            let path = format!("{base_path}{sep}{}", query.finish());

            let response = self.get(&path).await?;

            if let Some(results) = response.get("results").and_then(|v| v.as_array()) {
                for item in results {
                    if all_results.len() as u32 >= limit {
                        return Ok(all_results);
                    }
                    all_results.push(item.clone());
                }
            }

            if !opts.fetch_all && opts.limit.is_none() {
                break;
            }

            if all_results.len() as u32 >= limit {
                break;
            }

            let has_more = response
                .get("has_more")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !has_more {
                break;
            }

            cursor = response
                .get("next_cursor")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            match cursor {
                None => {
                    return Err(CliError::Pagination(
                        "Server indicated has_more=true but returned no next_cursor".into(),
                    ));
                }
                Some(ref new_c) if Some(new_c) == prev_cursor.as_ref() => {
                    return Err(CliError::Pagination(
                        "Server returned the same cursor twice".into(),
                    ));
                }
                _ => {}
            }

            prev_cursor = cursor.clone();
        }

        Ok(all_results)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use url::Url;
    use wiremock::{
        matchers::{any, method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    fn client_for(server: &MockServer) -> NotionClient {
        let base_url = Url::parse(&format!("{}/", server.uri())).unwrap();
        NotionClient::new("secret_test".to_string())
            .unwrap()
            .with_base_url(base_url)
    }

    #[tokio::test]
    async fn paginate_post_errors_on_missing_cursor() {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{"id": "1"}],
                "has_more": true,
                "next_cursor": null
            })))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let opts = PaginationOpts {
            fetch_all: true,
            ..Default::default()
        };
        let result = client.paginate_post("/v1/test", &json!({}), &opts).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("no next_cursor"),
            "expected missing cursor error, got: {err}"
        );
    }

    #[tokio::test]
    async fn paginate_post_errors_on_same_cursor_twice() {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{"id": "1"}],
                "has_more": true,
                "next_cursor": "same_cursor"
            })))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let opts = PaginationOpts {
            fetch_all: true,
            ..Default::default()
        };
        let result = client.paginate_post("/v1/test", &json!({}), &opts).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("same cursor twice"),
            "expected same cursor error, got: {err}"
        );
    }

    #[tokio::test]
    async fn paginate_post_collects_all_pages() {
        let server = MockServer::start().await;

        // Page 1: has_more=true with cursor
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{"id": "1"}, {"id": "2"}],
                "has_more": true,
                "next_cursor": "cursor_page2"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Page 2: has_more=false
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{"id": "3"}],
                "has_more": false,
                "next_cursor": null
            })))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let opts = PaginationOpts {
            fetch_all: true,
            ..Default::default()
        };
        let result = client
            .paginate_post("/v1/test", &json!({}), &opts)
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0]["id"], "1");
        assert_eq!(result[2]["id"], "3");
    }

    #[tokio::test]
    async fn paginate_post_respects_limit() {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{"id": "1"}, {"id": "2"}, {"id": "3"}],
                "has_more": false
            })))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let opts = PaginationOpts {
            limit: Some(2),
            ..Default::default()
        };
        let result = client
            .paginate_post("/v1/test", &json!({}), &opts)
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn paginate_get_errors_on_missing_cursor() {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{"id": "1"}],
                "has_more": true,
                "next_cursor": null
            })))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let opts = PaginationOpts {
            fetch_all: true,
            ..Default::default()
        };
        let result = client.paginate_get("/v1/test", &opts).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("no next_cursor"),
            "expected missing cursor error, got: {err}"
        );
    }

    #[tokio::test]
    async fn paginate_get_collects_all_pages() {
        let server = MockServer::start().await;

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{"id": "a"}],
                "has_more": true,
                "next_cursor": "c2"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{"id": "b"}],
                "has_more": false
            })))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let opts = PaginationOpts {
            fetch_all: true,
            ..Default::default()
        };
        let result = client.paginate_get("/v1/test", &opts).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn paginate_get_url_encodes_opaque_cursor() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/test"))
            .and(query_param("page_size", "50"))
            .and(query_param("start_cursor", "a&b+#%"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [],
                "has_more": false
            })))
            .expect(1)
            .mount(&server)
            .await;

        let opts = PaginationOpts {
            start_cursor: Some("a&b+#%".to_string()),
            ..Default::default()
        };
        client_for(&server)
            .paginate_get("/v1/test", &opts)
            .await
            .unwrap();
    }
}
