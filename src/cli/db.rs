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
    /// Query a database with optional filters
    Query {
        /// Database ID or URL
        id: String,
        /// Filter expression (DSL syntax)
        #[arg(long)]
        filter: Option<String>,
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
    /// List all databases (via search)
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
            id,
            filter: _filter_dsl,
            filter_json,
            sort,
            direction,
            all,
            limit,
            page_size,
            cursor,
        } => {
            let id = crate::normalize_id(&id);
            let filter = filter_json
                .map(|f| serde_json::from_str(&f))
                .transpose()?;
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
            let results = client.query_database(&id, filter, sorts, &pagination).await?;
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
                .search("", Some("database"), "descending", "last_edited_time", &pagination)
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
            let body = json!({
                "parent": { "page_id": parent_id },
                "title": [{ "type": "text", "text": { "content": title } }],
                "properties": props,
            });
            let db = client.post("/v1/databases", &body).await?;
            output::format_value(&db, format, &mut stdout)
        }
    }
}
