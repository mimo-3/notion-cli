use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub object: String,
    #[serde(rename = "type")]
    pub user_type: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person: Option<PersonData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonData {
    pub email: Option<String>,
}
