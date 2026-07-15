use serde_json::{json, Value};

use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;

impl NotionClient {
    pub async fn search(
        &self,
        query: &str,
        filter_object_type: Option<&str>,
        sort_direction: &str,
        sort_timestamp: &str,
        pagination: &PaginationOpts,
    ) -> Result<Vec<Value>, CliError> {
        let mut body = json!({
            "query": query,
            "sort": {
                "direction": sort_direction,
                "timestamp": sort_timestamp,
            },
        });

        if let Some(obj_type) = filter_object_type {
            body["filter"] = json!({
                "value": obj_type,
                "property": "object",
            });
        }

        self.paginate_post("/v1/search", &body, pagination).await
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use url::Url;
    use wiremock::{
        matchers::{body_json, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    #[tokio::test]
    async fn search_filters_for_data_sources() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/search"))
            .and(body_json(json!({
                "query": "",
                "sort": {
                    "direction": "descending",
                    "timestamp": "last_edited_time",
                },
                "filter": {"property": "object", "value": "data_source"},
                "page_size": 50,
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [],
                "has_more": false,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let base_url = Url::parse(&format!("{}/", server.uri())).unwrap();
        let client = NotionClient::new("secret_test".to_string())
            .unwrap()
            .with_base_url(base_url);
        client
            .search(
                "",
                Some("data_source"),
                "descending",
                "last_edited_time",
                &PaginationOpts::default(),
            )
            .await
            .unwrap();
    }
}
