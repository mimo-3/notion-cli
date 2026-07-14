use serde_json::Value;

use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;

impl NotionClient {
    pub async fn get_self(&self) -> Result<Value, CliError> {
        self.get("/v1/users/me").await
    }

    pub async fn get_user(&self, user_id: &str) -> Result<Value, CliError> {
        self.get(&format!("/v1/users/{user_id}")).await
    }

    pub async fn list_users(&self, pagination: &PaginationOpts) -> Result<Vec<Value>, CliError> {
        self.paginate_get("/v1/users", pagination).await
    }
}
