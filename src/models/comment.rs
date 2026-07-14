use serde::{Deserialize, Serialize};

use super::common::Parent;
use super::rich_text::RichText;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub object: String,
    pub parent: Parent,
    pub discussion_id: String,
    pub created_time: String,
    pub last_edited_time: String,
    pub created_by: serde_json::Value,
    pub rich_text: Vec<RichText>,
}
