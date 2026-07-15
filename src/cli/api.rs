use clap::Args;
use serde_json::{json, Value};

use crate::cli::GlobalOpts;
use crate::client::NotionClient;
use crate::error::CliError;
use crate::output;

#[derive(Args)]
pub struct ApiArgs {
    /// HTTP method (GET, POST, PATCH, DELETE)
    #[arg(long, default_value = "GET")]
    pub method: String,

    /// API endpoint path (e.g. /v1/pages/...)
    pub path: String,

    /// JSON request body
    #[arg(long)]
    pub data: Option<String>,

    /// Read request body from stdin
    #[arg(long)]
    pub stdin: bool,
}

pub async fn run(
    args: ApiArgs,
    client: &NotionClient,
    global: &GlobalOpts,
) -> Result<(), CliError> {
    let body: Option<Value> = if args.stdin {
        let input = std::io::read_to_string(std::io::stdin())?;
        Some(serde_json::from_str(&input)?)
    } else {
        args.data.map(|d| serde_json::from_str(&d)).transpose()?
    };

    let result = match args.method.to_uppercase().as_str() {
        "GET" => client.get(&args.path).await?,
        "POST" => client.post(&args.path, &body.unwrap_or(json!({}))).await?,
        "PATCH" => client.patch(&args.path, &body.unwrap_or(json!({}))).await?,
        "DELETE" => client.delete(&args.path).await?,
        other => {
            return Err(CliError::Config(format!(
                "Unsupported HTTP method: {other}"
            )));
        }
    };

    let format = global.output_format();
    let mut stdout = std::io::stdout();
    output::format_value(&result, format, &mut stdout)
}
