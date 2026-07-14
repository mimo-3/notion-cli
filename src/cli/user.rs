use clap::{Args, Subcommand};
use serde_json::Value;

use crate::cli::GlobalOpts;
use crate::client::pagination::PaginationOpts;
use crate::client::NotionClient;
use crate::error::CliError;
use crate::output;

#[derive(Args)]
pub struct UserCommand {
    #[command(subcommand)]
    pub command: UserSubcommand,
}

#[derive(Subcommand)]
pub enum UserSubcommand {
    /// Show the current user
    Me,
    /// List all users
    List {
        #[arg(long)]
        all: bool,
    },
    /// Get a user by ID
    Get { id: String },
}

pub async fn run(
    cmd: UserCommand,
    client: &NotionClient,
    global: &GlobalOpts,
) -> Result<(), CliError> {
    let format = global.output_format();
    let mut stdout = std::io::stdout();

    match cmd.command {
        UserSubcommand::Me => {
            let user = client.get_self().await?;
            output::format_value(&user, format, &mut stdout)
        }
        UserSubcommand::List { all } => {
            let opts = PaginationOpts {
                fetch_all: all,
                ..Default::default()
            };
            let users = client.list_users(&opts).await?;
            output::format_value(&Value::Array(users), format, &mut stdout)
        }
        UserSubcommand::Get { id } => {
            let user = client.get_user(&id).await?;
            output::format_value(&user, format, &mut stdout)
        }
    }
}
