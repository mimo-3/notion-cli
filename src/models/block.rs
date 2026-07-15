#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::common::Parent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub object: String,
    #[serde(rename = "type")]
    pub block_type: String,
    pub created_time: String,
    pub last_edited_time: String,
    pub parent: Parent,
    pub has_children: bool,
    pub archived: bool,
    /// The type-specific data is stored as a dynamic field matching block_type.
    #[serde(flatten)]
    pub data: serde_json::Value,
}
