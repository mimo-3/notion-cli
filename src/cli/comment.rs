use clap::{Args, Subcommand};
use serde_json::{json, Value};

use crate::cli::GlobalOpts;
use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;
use crate::output;

#[derive(Args)]
pub struct CommentCommand {
    #[command(subcommand)]
    pub command: CommentSubcommand,
}

#[derive(Subcommand)]
pub enum CommentSubcommand {
    /// List comments on a page or block
    List {
        /// Page or block ID
        id: String,
        #[arg(long)]
        all: bool,
    },
    /// Get a comment by ID
    Get {
        /// Comment ID
        id: String,
    },
    /// Add a comment
    Create {
        /// Parent page ID
        #[arg(long)]
        parent: String,
        /// Comment text
        text: String,
        /// Discussion ID (for replies)
        #[arg(long)]
        discussion: Option<String>,
    },
    /// Update a comment
    Update {
        /// Comment ID
        id: String,
        /// New comment text
        text: String,
    },
    /// Delete a comment
    Delete {
        /// Comment ID
        id: String,
    },
}

pub async fn run(
    cmd: CommentCommand,
    client: &NotionClient,
    global: &GlobalOpts,
) -> Result<(), CliError> {
    let format = global.output_format();
    let mut stdout = std::io::stdout();

    match cmd.command {
        CommentSubcommand::List { id, all } => {
            let id = crate::normalize_id(&id);
            let opts = PaginationOpts {
                fetch_all: all,
                ..Default::default()
            };
            let comments = client.list_comments(&id, &opts).await?;
            output::format_value(&Value::Array(comments), format, &mut stdout)
        }
        CommentSubcommand::Get { id } => {
            let result = client.get(&format!("/v1/comments/{id}")).await?;
            output::format_value(&result, format, &mut stdout)
        }
        CommentSubcommand::Create {
            parent,
            text,
            discussion,
        } => {
            let parent_id = crate::normalize_id(&parent);
            let mut body = json!({
                "parent": { "page_id": parent_id },
                "rich_text": [{
                    "type": "text",
                    "text": { "content": text },
                }],
            });
            if let Some(disc_id) = discussion {
                body["discussion_id"] = Value::String(disc_id);
            }
            let result = client.post("/v1/comments", &body).await?;
            output::format_value(&result, format, &mut stdout)
        }
        CommentSubcommand::Update { id, text } => {
            let body = json!({
                "rich_text": [{
                    "type": "text",
                    "text": { "content": text },
                }],
            });
            let result = client.patch(&format!("/v1/comments/{id}"), &body).await?;
            output::format_value(&result, format, &mut stdout)
        }
        CommentSubcommand::Delete { id } => {
            let result = client.delete(&format!("/v1/comments/{id}")).await?;
            eprintln!("Comment {id} deleted.");
            output::format_value(&result, format, &mut stdout)
        }
    }
}
