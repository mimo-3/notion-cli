use serde_json::{json, Value};

use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;

impl NotionClient {
    pub async fn list_comments(
        &self,
        block_id: &str,
        pagination: &PaginationOpts,
    ) -> Result<Vec<Value>, CliError> {
        self.paginate_get(
            &format!("/v1/comments?block_id={block_id}"),
            pagination,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn create_comment(
        &self,
        parent_id: &str,
        text: &str,
    ) -> Result<Value, CliError> {
        let body = json!({
            "parent": { "page_id": parent_id },
            "rich_text": [{
                "type": "text",
                "text": { "content": text },
            }],
        });
        self.post("/v1/comments", &body).await
    }
}
