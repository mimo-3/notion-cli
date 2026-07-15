use clap::{Args, Subcommand};
use serde_json::Value;

use crate::api::files::detect_content_type;
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
        FileSubcommand::Upload { path } => {
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

            output::format_value(&result, format, &mut stdout)
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
