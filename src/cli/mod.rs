pub mod api;
pub mod auth;
pub mod block;
pub mod comment;
pub mod config_cmd;
pub mod db;
pub mod file;
pub mod page;
pub mod search;
pub mod user;
pub mod view;

use clap::{Args, Parser, Subcommand};

use crate::output::OutputFormat;

#[derive(Parser)]
#[command(name = "notion", version, about = "Command-line interface for the Notion API")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    pub global: GlobalOpts,
}

#[derive(Args, Debug)]
pub struct GlobalOpts {
    /// Notion API token (overrides env and config)
    #[arg(long, env = "NOTION_API_TOKEN", global = true, hide_env_values = true)]
    pub token: Option<String>,

    /// Config profile to use
    #[arg(long, global = true)]
    pub profile: Option<String>,

    /// Override Notion API version
    #[arg(long, global = true)]
    pub api_version: Option<String>,

    /// Output format
    #[arg(long, global = true, default_value = "plain")]
    pub format: OutputFormat,

    /// Shorthand for --format json
    #[arg(long, global = true)]
    pub json: bool,

    /// Enable verbose output
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Show what would be done without making changes
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,
}

impl GlobalOpts {
    /// Resolve the effective output format, considering --json shorthand.
    pub fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else {
            self.format
        }
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Manage authentication
    Auth(auth::AuthCommand),
    /// Search across all pages and databases
    Search(search::SearchArgs),
    /// Work with pages
    Page(page::PageCommand),
    /// Work with databases
    Db(db::DbCommand),
    /// Work with blocks
    Block(block::BlockCommand),
    /// Work with comments
    Comment(comment::CommentCommand),
    /// Work with users
    User(user::UserCommand),
    /// Work with files
    File(file::FileCommand),
    /// Work with views
    View(view::ViewCommand),
    /// Make raw API calls
    Api(api::ApiArgs),
    /// Manage CLI configuration
    Config(config_cmd::ConfigCommand),
}
