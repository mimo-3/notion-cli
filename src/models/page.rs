use serde::{Deserialize, Serialize};

use super::common::{Cover, Icon, Parent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    pub object: String,
    pub created_time: String,
    pub last_edited_time: String,
    pub created_by: serde_json::Value,
    pub last_edited_by: serde_json::Value,
    pub parent: Parent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Cover>,
    pub properties: serde_json::Value,
    pub archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_url: Option<String>,
}
