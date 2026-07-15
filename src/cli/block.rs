use clap::{Args, Subcommand};
use serde_json::Value;

use crate::cli::GlobalOpts;
use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;
use crate::output;

#[derive(Args)]
pub struct BlockCommand {
    #[command(subcommand)]
    pub command: BlockSubcommand,
}

#[derive(Subcommand)]
pub enum BlockSubcommand {
    /// Retrieve a block by ID
    Get { id: String },
    /// List child blocks
    Children {
        id: String,
        #[arg(long)]
        all: bool,
        /// Recursively fetch all descendant blocks
        #[arg(long)]
        recursive: bool,
    },
    /// Append child blocks
    Append {
        id: String,
        /// JSON block children from stdin
        #[arg(long)]
        stdin: bool,
    },
    /// Update a block
    Update {
        id: String,
        /// Block data as JSON (reads from stdin if not provided)
        #[arg(long)]
        data: Option<String>,
    },
    /// Delete a block
    Delete { id: String },
}

pub async fn run(
    cmd: BlockCommand,
    client: &NotionClient,
    global: &GlobalOpts,
) -> Result<(), CliError> {
    let format = global.output_format();
    let mut stdout = std::io::stdout();

    match cmd.command {
        BlockSubcommand::Get { id } => {
            let id = crate::normalize_id(&id);
            let block = client.get_block(&id).await?;
            output::format_value(&block, format, &mut stdout)
        }
        BlockSubcommand::Children { id, all, recursive } => {
            let id = crate::normalize_id(&id);
            let opts = PaginationOpts {
                page_size: 100,
                fetch_all: all || recursive,
                ..Default::default()
            };
            let blocks = client.get_block_children(&id, &opts).await?;

            if recursive {
                let mut all_blocks = Vec::new();
                let mut stack: Vec<Value> = blocks;
                while let Some(block) = stack.pop() {
                    let has_children = block
                        .get("has_children")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if has_children {
                        if let Some(block_id) = block.get("id").and_then(|v| v.as_str()) {
                            let child_opts = PaginationOpts {
                                page_size: 100,
                                fetch_all: true,
                                ..Default::default()
                            };
                            let children = client.get_block_children(block_id, &child_opts).await?;
                            for child in children.into_iter().rev() {
                                stack.push(child);
                            }
                        }
                    }
                    all_blocks.push(block);
                }
                output::format_value(&Value::Array(all_blocks), format, &mut stdout)
            } else {
                output::format_value(&Value::Array(blocks), format, &mut stdout)
            }
        }
        BlockSubcommand::Append { id, stdin: _ } => {
            let id = crate::normalize_id(&id);
            let input = std::io::read_to_string(std::io::stdin())?;
            let children: Vec<Value> = serde_json::from_str(&input)?;
            let result = client.append_block_children(&id, children).await?;
            output::format_value(&result, format, &mut stdout)
        }
        BlockSubcommand::Update { id, data } => {
            let id = crate::normalize_id(&id);
            let body: Value = if let Some(data_str) = data {
                serde_json::from_str(&data_str)?
            } else {
                let input = std::io::read_to_string(std::io::stdin())?;
                serde_json::from_str(&input)?
            };
            let result = client.patch(&format!("/v1/blocks/{id}"), &body).await?;
            output::format_value(&result, format, &mut stdout)
        }
        BlockSubcommand::Delete { id } => {
            let id = crate::normalize_id(&id);
            let result = client.delete_block(&id).await?;
            output::format_value(&result, format, &mut stdout)
        }
    }
}
