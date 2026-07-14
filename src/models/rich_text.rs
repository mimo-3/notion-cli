use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichText {
    #[serde(rename = "type")]
    pub rich_text_type: String,
    pub plain_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<Link>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotations {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub underline: bool,
    pub code: bool,
    pub color: String,
}

impl RichText {
    /// Create a simple text RichText.
    pub fn text(content: &str) -> Self {
        Self {
            rich_text_type: "text".to_string(),
            plain_text: content.to_string(),
            href: None,
            annotations: None,
            text: Some(TextContent {
                content: content.to_string(),
                link: None,
            }),
        }
    }
}
