use serde_json::{json, Value};

use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;

impl NotionClient {
    pub async fn get_database(&self, database_id: &str) -> Result<Value, CliError> {
        self.get(&format!("/v1/databases/{database_id}")).await
    }

    pub async fn query_database(
        &self,
        database_id: &str,
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
            &format!("/v1/databases/{database_id}/query"),
            &body,
            pagination,
        )
        .await
    }
}
