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
