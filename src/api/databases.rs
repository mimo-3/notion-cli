use serde_json::{json, Value};

use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;

impl NotionClient {
    pub async fn get_database(&self, database_id: &str) -> Result<Value, CliError> {
        self.get(&format!("/v1/databases/{database_id}")).await
    }

    pub async fn query_data_source(
        &self,
        data_source_id: &str,
        filter: Option<Value>,
        sorts: Option<Vec<Value>>,
        pagination: &PaginationOpts,
    ) -> Result<Vec<Value>, CliError> {
        let mut body = json!({});
        if let Some(f) = filter {
            body["filter"] = f;
        }
        if let Some(s) = sorts {
            body["sorts"] = Value::Array(s);
        }
        self.paginate_post(
            &format!("/v1/data_sources/{data_source_id}/query"),
            &body,
            pagination,
        )
        .await
    }

    pub async fn create_database(
        &self,
        parent_page_id: &str,
        title: &str,
        initial_properties: Value,
    ) -> Result<Value, CliError> {
        let body = json!({
            "parent": {
                "type": "page_id",
                "page_id": parent_page_id,
            },
            "title": [{ "type": "text", "text": { "content": title } }],
            "initial_data_source": {
                "properties": initial_properties,
            },
        });
        self.post("/v1/databases", &body).await
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

    fn client_for(server: &MockServer) -> NotionClient {
        let base_url = Url::parse(&format!("{}/", server.uri())).unwrap();
        NotionClient::new("secret_test".to_string())
            .unwrap()
            .with_base_url(base_url)
    }

    #[tokio::test]
    async fn query_data_source_uses_data_source_endpoint_and_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/data_sources/source-123/query"))
            .and(body_json(json!({
                "filter": {"property": "Status", "select": {"equals": "Open"}},
                "sorts": [{"property": "Name", "direction": "ascending"}],
                "page_size": 25,
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [],
                "has_more": false,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let pagination = PaginationOpts {
            page_size: 25,
            ..Default::default()
        };
        client_for(&server)
            .query_data_source(
                "source-123",
                Some(json!({"property": "Status", "select": {"equals": "Open"}})),
                Some(vec![json!({"property": "Name", "direction": "ascending"})]),
                &pagination,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_database_nests_properties_under_initial_data_source() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/databases"))
            .and(body_json(json!({
                "parent": {"type": "page_id", "page_id": "page-123"},
                "title": [{"type": "text", "text": {"content": "Tasks"}}],
                "initial_data_source": {
                    "properties": {"Name": {"title": {}}},
                },
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "db-123"})))
            .expect(1)
            .mount(&server)
            .await;

        client_for(&server)
            .create_database("page-123", "Tasks", json!({"Name": {"title": {}}}))
            .await
            .unwrap();
    }
}
