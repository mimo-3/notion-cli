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
