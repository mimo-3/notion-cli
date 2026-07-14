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

    pub async fn update_page(
        &self,
        page_id: &str,
        properties: Option<Value>,
        archived: Option<bool>,
    ) -> Result<Value, CliError> {
        let mut body = json!({});
        if let Some(props) = properties {
            body["properties"] = props;
        }
        if let Some(arch) = archived {
            body["archived"] = Value::Bool(arch);
        }
        self.patch(&format!("/v1/pages/{page_id}"), &body).await
    }
}
