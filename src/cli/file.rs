use clap::{Args, Subcommand};
use serde_json::Value;

use crate::api::files::{
    block_type_for_content_type, detect_content_type, AFTER_CAPABLE_API_VERSION,
};
use crate::cli::GlobalOpts;
use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;
use crate::output;

#[derive(Args)]
pub struct FileCommand {
    #[command(subcommand)]
    pub command: FileSubcommand,
}

#[derive(Subcommand)]
pub enum FileSubcommand {
    /// Upload a file to Notion
    Upload {
        /// Path to the file
        path: String,
        /// Page or block ID to attach the uploaded file to as a child block
        #[arg(long)]
        parent: Option<String>,
        /// Existing child block ID to insert the new block after (requires --parent)
        #[arg(long, requires = "parent")]
        after: Option<String>,
    },
    /// Get file upload metadata
    Get {
        /// File upload ID
        id: String,
    },
    /// List file uploads
    List {
        /// Fetch all pages
        #[arg(long)]
        all: bool,
        /// Maximum number of results
        #[arg(long)]
        limit: Option<u32>,
    },
}

pub async fn run(
    cmd: FileCommand,
    client: &NotionClient,
    global: &GlobalOpts,
) -> Result<(), CliError> {
    let format = global.output_format();
    let mut stdout = std::io::stdout();

    match cmd.command {
        FileSubcommand::Upload {
            path,
            parent,
            after,
        } => {
            let file_path = std::path::Path::new(&path);
            if !file_path.exists() {
                return Err(CliError::Config(format!("File not found: {path}")));
            }

            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");

            let content_type = detect_content_type(filename);
            let file_size = std::fs::metadata(file_path)?.len();

            eprintln!("File: {} ({} bytes, {})", filename, file_size, content_type);

            let result = client
                .upload_file_path(file_path, filename, content_type)
                .await?;

            let Some(parent) = parent else {
                return output::format_value(&result, format, &mut stdout);
            };

            let parent_id = crate::normalize_id(&parent);

            if global.dry_run {
                eprintln!("[dry-run] PATCH /v1/blocks/{parent_id}/children");
                return output::format_value(&result, format, &mut stdout);
            }

            let file_upload_id = result
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::Config("Missing file upload id in response".to_string()))?
                .to_string();

            let block_type = block_type_for_content_type(content_type);
            let after_id = after.map(|a| crate::normalize_id(&a));

            // `after` only exists in the 2022-06-28 API contract; pin it for this
            // request unless the user chose a version explicitly.
            let pinned_version = match (&after_id, &global.api_version) {
                (Some(_), None) => {
                    eprintln!(
                        "Using Notion-Version {AFTER_CAPABLE_API_VERSION} for positioned attach (--after)"
                    );
                    Some(AFTER_CAPABLE_API_VERSION)
                }
                _ => None,
            };

            eprintln!("Attaching to {parent_id} as {block_type} block...");
            let attached = client
                .attach_file_upload(
                    &parent_id,
                    &file_upload_id,
                    block_type,
                    after_id.as_deref(),
                    pinned_version,
                )
                .await?;

            output::format_value(&attached, format, &mut stdout)
        }
        FileSubcommand::Get { id } => {
            let file_id = crate::normalize_id(&id);
            let result = client.get_file_upload(&file_id).await?;
            output::format_value(&result, format, &mut stdout)
        }
        FileSubcommand::List { all, limit } => {
            let opts = PaginationOpts {
                fetch_all: all,
                limit,
                ..Default::default()
            };
            let results = client.list_file_uploads(&opts).await?;
            output::format_value(&Value::Array(results), format, &mut stdout)
        }
    }
}
