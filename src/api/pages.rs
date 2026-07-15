use serde_json::{json, Value};

use crate::client::NotionClient;
use crate::error::CliError;

impl NotionClient {
    pub async fn get_page(&self, page_id: &str) -> Result<Value, CliError> {
        self.get(&format!("/v1/pages/{page_id}")).await
    }

    pub async fn create_page(
        &self,
        parent: Value,
        properties: Value,
        children: Option<Vec<Value>>,
    ) -> Result<Value, CliError> {
        let mut body = json!({
            "parent": parent,
            "properties": properties,
        });
        if let Some(kids) = children {
            body["children"] = Value::Array(kids);
        }
        self.post("/v1/pages", &body).await
    }

    pub async fn move_page(&self, page_id: &str, parent: Value) -> Result<Value, CliError> {
        self.post(
            &format!("/v1/pages/{page_id}/move"),
            &json!({ "parent": parent }),
        )
        .await
    }

    /// Get page content as markdown (Notion's native markdown endpoint).
    pub async fn get_page_markdown(&self, page_id: &str) -> Result<String, CliError> {
        let value = self.get(&format!("/v1/pages/{page_id}/markdown")).await?;
        Ok(value
            .get("markdown")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    /// Update page content with markdown (PATCH replace_content).
    pub async fn update_page_markdown(
        &self,
        page_id: &str,
        markdown: &str,
    ) -> Result<Value, CliError> {
        let body = json!({
            "type": "replace_content",
            "replace_content": {
                "new_str": markdown
            }
        });
        self.patch(&format!("/v1/pages/{page_id}/markdown"), &body)
            .await
    }

    pub async fn update_page(
        &self,
        page_id: &str,
        properties: Option<Value>,
        in_trash: Option<bool>,
    ) -> Result<Value, CliError> {
        let mut body = json!({});
        if let Some(props) = properties {
            body["properties"] = props;
        }
        if let Some(trash) = in_trash {
            body["in_trash"] = Value::Bool(trash);
        }
        self.patch(&format!("/v1/pages/{page_id}"), &body).await
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
    async fn create_page_preserves_data_source_parent_contract() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .and(body_json(json!({
                "parent": {"type": "data_source_id", "data_source_id": "source-123"},
                "properties": {"title": {"title": []}},
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "page-123"})))
            .expect(1)
            .mount(&server)
            .await;

        client_for(&server)
            .create_page(
                json!({"type": "data_source_id", "data_source_id": "source-123"}),
                json!({"title": {"title": []}}),
                None,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn move_page_uses_typed_data_source_parent() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/pages/page-123/move"))
            .and(body_json(json!({
                "parent": {"type": "data_source_id", "data_source_id": "source-123"},
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "page-123"})))
            .expect(1)
            .mount(&server)
            .await;

        client_for(&server)
            .move_page(
                "page-123",
                json!({"type": "data_source_id", "data_source_id": "source-123"}),
            )
            .await
            .unwrap();
    }
}
