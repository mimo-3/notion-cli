use clap::Args;
use serde_json::Value;

use crate::cli::GlobalOpts;
use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;
use crate::output;

#[derive(Args)]
pub struct SearchArgs {
    /// Search query text
    pub query: String,

    /// Filter by object type: page or database
    #[arg(long, value_name = "TYPE")]
    pub filter: Option<String>,

    /// Sort direction: ascending or descending
    #[arg(long, default_value = "descending")]
    pub sort: String,

    /// Sort by timestamp field: last_edited_time
    #[arg(long, default_value = "last_edited_time")]
    pub sort_by: String,

    /// Fetch all results (auto-paginate)
    #[arg(long)]
    pub all: bool,

    /// Maximum number of results
    #[arg(long)]
    pub limit: Option<u32>,

    /// Page size per request (1-100)
    #[arg(long, default_value = "50")]
    pub page_size: u8,

    /// Pagination cursor
    #[arg(long)]
    pub cursor: Option<String>,
}

pub async fn run(
    args: SearchArgs,
    client: &NotionClient,
    global: &GlobalOpts,
) -> Result<(), CliError> {
    let pagination = PaginationOpts {
        page_size: args.page_size,
        start_cursor: args.cursor,
        fetch_all: args.all,
        limit: args.limit,
    };

    let results = client
        .search(
            &args.query,
            args.filter.as_deref(),
            &args.sort,
            &args.sort_by,
            &pagination,
        )
        .await?;

    let format = global.output_format();
    let mut stdout = std::io::stdout();
    let value = Value::Array(results);
    output::format_value(&value, format, &mut stdout)?;
    Ok(())
}
