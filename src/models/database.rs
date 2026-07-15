#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::common::{Cover, Icon, Parent};
use super::rich_text::RichText;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    pub id: String,
    pub object: String,
    pub created_time: String,
    pub last_edited_time: String,
    pub title: Vec<RichText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Vec<RichText>>,
    pub parent: Parent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Cover>,
    pub properties: serde_json::Value,
    pub archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub is_inline: bool,
}
