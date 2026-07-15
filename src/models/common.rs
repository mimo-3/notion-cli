#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Parent {
    DatabaseId { database_id: String },
    PageId { page_id: String },
    BlockId { block_id: String },
    Workspace { workspace: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Icon {
    Emoji { emoji: String },
    External { external: ExternalFile },
    File { file: NotionFile },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalFile {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionFile {
    pub url: String,
    pub expiry_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Cover {
    External { external: ExternalFile },
    File { file: NotionFile },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub object: String,
    pub results: Vec<T>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
    #[serde(rename = "type")]
    pub result_type: Option<String>,
}
