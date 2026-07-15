use serde_json::{json, Value};

use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;

impl NotionClient {
    pub async fn get_block(&self, block_id: &str) -> Result<Value, CliError> {
        self.get(&format!("/v1/blocks/{block_id}")).await
    }

    pub async fn get_block_children(
        &self,
        block_id: &str,
        pagination: &PaginationOpts,
    ) -> Result<Vec<Value>, CliError> {
        self.paginate_get(&format!("/v1/blocks/{block_id}/children"), pagination)
            .await
    }

    pub async fn append_block_children(
        &self,
        block_id: &str,
        children: Vec<Value>,
    ) -> Result<Value, CliError> {
        let body = json!({ "children": children });
        self.patch(&format!("/v1/blocks/{block_id}/children"), &body)
            .await
    }

    pub async fn delete_block(&self, block_id: &str) -> Result<Value, CliError> {
        self.delete(&format!("/v1/blocks/{block_id}")).await
    }
}
