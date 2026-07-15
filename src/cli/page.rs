use std::io::{Read, Write};

use clap::{Args, Subcommand, ValueEnum};
use serde_json::{json, Value};

use crate::cli::GlobalOpts;
use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;
use crate::output;

#[derive(Args)]
pub struct PageCommand {
    #[command(subcommand)]
    pub command: PageSubcommand,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum PageParentType {
    /// A regular Notion page
    Page,
    /// A database data source (use the data source ID, not the database ID)
    DataSource,
}

impl PageParentType {
    fn parent(self, id: String) -> Value {
        match self {
            Self::Page => json!({ "type": "page_id", "page_id": id }),
            Self::DataSource => {
                json!({ "type": "data_source_id", "data_source_id": id })
            }
        }
    }
}

#[derive(Subcommand)]
pub enum PageSubcommand {
    /// Retrieve a page by ID
    Get {
        /// Page ID or URL
        id: String,
    },
    /// Get page content (blocks)
    Content {
        /// Page ID or URL
        id: String,
        /// Fetch all blocks (auto-paginate)
        #[arg(long)]
        all: bool,
        /// Output as Markdown (implies --all)
        #[arg(long)]
        markdown: bool,
    },
    /// Edit page content with Markdown from stdin
    Edit {
        /// Page ID or URL
        id: String,
    },
    /// Create a new page
    Create {
        /// Parent page or data source ID
        #[arg(long)]
        parent: String,
        /// Page title
        #[arg(long)]
        title: String,
        /// Read content from stdin as JSON blocks
        #[arg(long)]
        stdin: bool,
        /// Parent type: page or data-source
        #[arg(long, value_enum, default_value_t = PageParentType::Page)]
        parent_type: PageParentType,
    },
    /// Update page properties
    Update {
        /// Page ID or URL
        id: String,
        /// JSON properties to set
        #[arg(long)]
        properties: Option<String>,
        /// Set page icon (emoji)
        #[arg(long)]
        icon: Option<String>,
        /// Set page cover URL
        #[arg(long)]
        cover: Option<String>,
        /// Archive the page
        #[arg(long)]
        archive: bool,
    },
    /// Soft-delete a page (move to trash)
    Trash {
        /// Page ID or URL
        id: String,
    },
    /// Restore a page from trash
    Restore {
        /// Page ID or URL
        id: String,
    },
    /// Move a page to a new parent
    Move {
        /// Page ID
        id: String,
        /// New parent page or data source ID
        #[arg(long)]
        parent: String,
        /// Parent type: page or data-source
        #[arg(long, value_enum, default_value_t = PageParentType::Page)]
        parent_type: PageParentType,
    },
}

pub async fn run(
    cmd: PageCommand,
    client: &NotionClient,
    global: &GlobalOpts,
) -> Result<(), CliError> {
    let format = global.output_format();
    let mut stdout = std::io::stdout();

    match cmd.command {
        PageSubcommand::Get { id } => {
            let id = crate::normalize_id(&id);
            let page = client.get_page(&id).await?;
            output::format_value(&page, format, &mut stdout)
        }
        PageSubcommand::Content { id, all, markdown } => {
            let id = crate::normalize_id(&id);
            if markdown {
                // Try native markdown endpoint first
                match client.get_page_markdown(&id).await {
                    Ok(md) if !md.is_empty() => {
                        write!(stdout, "{md}")?;
                        Ok(())
                    }
                    _ => {
                        // Fallback: fetch all blocks and convert
                        let opts = PaginationOpts {
                            page_size: 100,
                            fetch_all: true,
                            ..Default::default()
                        };
                        let blocks = client.get_block_children(&id, &opts).await?;
                        output::format_value(
                            &Value::Array(blocks),
                            output::OutputFormat::Markdown,
                            &mut stdout,
                        )
                    }
                }
            } else {
                let opts = PaginationOpts {
                    page_size: 100,
                    fetch_all: all,
                    ..Default::default()
                };
                let blocks = client.get_block_children(&id, &opts).await?;
                output::format_value(&Value::Array(blocks), format, &mut stdout)
            }
        }
        PageSubcommand::Edit { id } => {
            let id = crate::normalize_id(&id);
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            let result = client.update_page_markdown(&id, &input).await?;
            output::format_value(&result, format, &mut stdout)
        }
        PageSubcommand::Create {
            parent,
            title,
            stdin: from_stdin,
            parent_type,
        } => {
            let parent_id = crate::normalize_id(&parent);
            let parent_val = parent_type.parent(parent_id);
            let properties = if matches!(parent_type, PageParentType::DataSource) {
                json!({
                    "title": {
                        "title": [{ "type": "text", "text": { "content": title } }]
                    }
                })
            } else {
                json!({
                    "title": [{ "type": "text", "text": { "content": title } }]
                })
            };
            let children = if from_stdin {
                let mut input = String::new();
                std::io::stdin().read_to_string(&mut input)?;
                let parsed: Vec<Value> = serde_json::from_str(&input)?;
                Some(parsed)
            } else {
                None
            };
            let page = client.create_page(parent_val, properties, children).await?;
            output::format_value(&page, format, &mut stdout)
        }
        PageSubcommand::Update {
            id,
            properties,
            icon,
            cover,
            archive,
        } => {
            let id = crate::normalize_id(&id);
            let mut body = json!({});
            if let Some(props_str) = properties {
                let props: Value = serde_json::from_str(&props_str)?;
                body["properties"] = props;
            }
            if let Some(emoji) = icon {
                body["icon"] = json!({ "type": "emoji", "emoji": emoji });
            }
            if let Some(cover_url) = cover {
                body["cover"] = json!({ "type": "external", "external": { "url": cover_url } });
            }
            if archive {
                body["in_trash"] = Value::Bool(true);
            }
            let page = client.patch(&format!("/v1/pages/{id}"), &body).await?;
            output::format_value(&page, format, &mut stdout)
        }
        PageSubcommand::Trash { id } => {
            let id = crate::normalize_id(&id);
            let page = client.update_page(&id, None, Some(true)).await?;
            eprintln!("Page {id} moved to trash.");
            output::format_value(&page, format, &mut stdout)
        }
        PageSubcommand::Restore { id } => {
            let id = crate::normalize_id(&id);
            let page = client.update_page(&id, None, Some(false)).await?;
            eprintln!("Page {id} restored from trash.");
            output::format_value(&page, format, &mut stdout)
        }
        PageSubcommand::Move {
            id,
            parent,
            parent_type,
        } => {
            let id = crate::normalize_id(&id);
            let parent_id = crate::normalize_id(&parent);
            let parent_val = parent_type.parent(parent_id);
            let result = client.move_page(&id, parent_val).await?;
            eprintln!("Page {id} moved.");
            output::format_value(&result, format, &mut stdout)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_source_parent_uses_typed_data_source_id() {
        assert_eq!(
            PageParentType::DataSource.parent("source-123".to_string()),
            json!({"type": "data_source_id", "data_source_id": "source-123"})
        );
    }

    #[test]
    fn page_parent_uses_typed_page_id() {
        assert_eq!(
            PageParentType::Page.parent("page-123".to_string()),
            json!({"type": "page_id", "page_id": "page-123"})
        );
    }
}
