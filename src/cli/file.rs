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
        /// Page or block ID to attach the uploaded file to as a child block.
        /// Prints the attach response; the file upload ID goes to stderr
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

/// Decide which Notion API version the attach request should pin, and whether
/// to warn the user.
///
/// The `after` parameter only exists in the 2022-06-28 API contract. Without an
/// explicit `--api-version` we pin that version automatically; an explicit
/// choice is respected, but anything other than 2022-06-28 combined with
/// `--after` is likely to be rejected by the API, so it earns a warning.
fn resolve_attach_api_version(
    after: Option<&str>,
    explicit_api_version: Option<&str>,
) -> (Option<&'static str>, Option<String>) {
    match (after, explicit_api_version) {
        (Some(_), None) => (Some(AFTER_CAPABLE_API_VERSION), None),
        (Some(_), Some(version)) if version != AFTER_CAPABLE_API_VERSION => (
            None,
            Some(format!(
                "--after is only accepted by API version {AFTER_CAPABLE_API_VERSION}; \
                 the request may be rejected by {version}"
            )),
        ),
        _ => (None, None),
    }
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

            // In dry-run mode the upload never happens, so there is no real id;
            // use a placeholder so the attach request can still be previewed.
            let file_upload_id = if global.dry_run {
                "<file-upload-id>".to_string()
            } else {
                result
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        CliError::Config("Missing file upload id in response".to_string())
                    })?
                    .to_string()
            };

            let block_type = block_type_for_content_type(content_type);
            let after_id = after.map(|a| crate::normalize_id(&a));

            let (pinned_version, warning) =
                resolve_attach_api_version(after_id.as_deref(), global.api_version.as_deref());
            if let Some(warning) = warning {
                eprintln!("warning: {warning}");
            }
            if pinned_version.is_some() {
                eprintln!(
                    "Using Notion-Version {AFTER_CAPABLE_API_VERSION} for positioned attach (--after)"
                );
            }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_version_pins_only_when_after_is_set_without_explicit_version() {
        assert_eq!(
            resolve_attach_api_version(Some("block_1"), None),
            (Some(AFTER_CAPABLE_API_VERSION), None)
        );
        assert_eq!(resolve_attach_api_version(None, None), (None, None));
        assert_eq!(
            resolve_attach_api_version(None, Some("2026-03-11")),
            (None, None)
        );
    }

    #[test]
    fn attach_version_respects_explicit_after_capable_version_silently() {
        assert_eq!(
            resolve_attach_api_version(Some("block_1"), Some(AFTER_CAPABLE_API_VERSION)),
            (None, None)
        );
    }

    #[test]
    fn attach_version_warns_when_explicit_version_cannot_take_after() {
        let (pinned, warning) = resolve_attach_api_version(Some("block_1"), Some("2026-03-11"));
        assert_eq!(pinned, None);
        let warning = warning.expect("a warning should be produced");
        assert!(warning.contains(AFTER_CAPABLE_API_VERSION));
        assert!(warning.contains("2026-03-11"));
    }
}
