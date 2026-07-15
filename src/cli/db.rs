use clap::{Args, Subcommand};
use serde_json::{json, Value};

use crate::cli::GlobalOpts;
use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;
use crate::output;

#[derive(Args)]
pub struct DbCommand {
    #[command(subcommand)]
    pub command: DbSubcommand,
}

#[derive(Subcommand)]
pub enum DbSubcommand {
    /// Retrieve a database by ID
    Get {
        /// Database ID or URL
        id: String,
    },
    /// Query a data source with optional filters
    Query {
        /// Data source ID or URL (not the database ID)
        data_source_id: String,
        /// Filter as raw JSON
        #[arg(long)]
        filter_json: Option<String>,
        /// Sort property
        #[arg(long)]
        sort: Option<String>,
        /// Sort direction: ascending or descending
        #[arg(long, default_value = "ascending")]
        direction: String,
        /// Fetch all results
        #[arg(long)]
        all: bool,
        /// Max number of results
        #[arg(long)]
        limit: Option<u32>,
        /// Page size (1-100)
        #[arg(long, default_value = "50")]
        page_size: u8,
        /// Pagination cursor
        #[arg(long)]
        cursor: Option<String>,
    },
    /// List all data sources (via search)
    List {
        /// Fetch all results
        #[arg(long)]
        all: bool,
        /// Max number of results
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Create a new database
    Create {
        /// Parent page ID
        #[arg(long)]
        parent: String,
        /// Database title
        #[arg(long)]
        title: String,
        /// Properties schema as JSON
        #[arg(long)]
        properties: Option<String>,
    },
}

pub async fn run(
    cmd: DbCommand,
    client: &NotionClient,
    global: &GlobalOpts,
) -> Result<(), CliError> {
    let format = global.output_format();
    let mut stdout = std::io::stdout();

    match cmd.command {
        DbSubcommand::Get { id } => {
            let id = crate::normalize_id(&id);
            let db = client.get_database(&id).await?;
            output::format_value(&db, format, &mut stdout)
        }
        DbSubcommand::Query {
            data_source_id,
            filter_json,
            sort,
            direction,
            all,
            limit,
            page_size,
            cursor,
        } => {
            let data_source_id = crate::normalize_id(&data_source_id);
            let filter = filter_json.map(|f| serde_json::from_str(&f)).transpose()?;
            let sorts = sort.map(|s| {
                vec![json!({
                    "property": s,
                    "direction": direction,
                })]
            });
            let pagination = PaginationOpts {
                page_size,
                start_cursor: cursor,
                fetch_all: all,
                limit,
            };
            let results = client
                .query_data_source(&data_source_id, filter, sorts, &pagination)
                .await?;
            output::format_value(&Value::Array(results), format, &mut stdout)
        }
        DbSubcommand::List { all, limit } => {
            let pagination = PaginationOpts {
                page_size: 50,
                start_cursor: None,
                fetch_all: all,
                limit,
            };
            let results = client
                .search(
                    "",
                    Some("data_source"),
                    "descending",
                    "last_edited_time",
                    &pagination,
                )
                .await?;
            output::format_value(&Value::Array(results), format, &mut stdout)
        }
        DbSubcommand::Create {
            parent,
            title,
            properties,
        } => {
            let parent_id = crate::normalize_id(&parent);
            let props: Value = if let Some(props_str) = properties {
                serde_json::from_str(&props_str)?
            } else {
                // Minimal: just a title property
                json!({
                    "Name": { "title": {} }
                })
            };
            let db = client.create_database(&parent_id, &title, props).await?;
            output::format_value(&db, format, &mut stdout)
        }
    }
}
