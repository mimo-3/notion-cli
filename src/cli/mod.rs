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
use std::io::IsTerminal;

use crate::output::OutputFormat;

#[derive(Parser)]
#[command(
    name = "notion",
    version,
    about = "Command-line interface for the Notion API"
)]
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
    #[arg(long, global = true)]
    pub format: Option<OutputFormat>,

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
    /// Resolve the effective output format for standard commands.
    pub fn output_format(&self) -> OutputFormat {
        self.resolve_output_format(std::io::stdout().is_terminal(), false)
    }

    /// Resolve the effective output format for raw API calls.
    pub fn api_output_format(&self) -> OutputFormat {
        self.resolve_output_format(std::io::stdout().is_terminal(), true)
    }

    fn resolve_output_format(&self, is_terminal: bool, is_api: bool) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else if let Some(format) = self.format {
            format
        } else if is_api || !is_terminal {
            OutputFormat::Json
        } else {
            OutputFormat::Plain
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

#[cfg(test)]
mod tests {
    use clap::{error::ErrorKind, Parser};

    use super::*;

    #[test]
    fn output_format_defaults_and_explicit_options() {
        let parse = |args: &[&str]| Cli::try_parse_from(args).expect("valid arguments").global;
        let default = parse(&["notion", "search", "test"]);
        assert_eq!(default.format, None);
        assert_eq!(
            default.resolve_output_format(true, false),
            OutputFormat::Plain
        );
        assert_eq!(
            default.resolve_output_format(false, false),
            OutputFormat::Json
        );
        assert_eq!(
            default.resolve_output_format(true, true),
            OutputFormat::Json
        );
        assert_eq!(
            default.resolve_output_format(false, true),
            OutputFormat::Json
        );

        for is_terminal in [true, false] {
            for is_api in [true, false] {
                let plain = parse(&["notion", "api", "/v1/pages", "--format", "plain"]);
                assert_eq!(
                    plain.resolve_output_format(is_terminal, is_api),
                    OutputFormat::Plain
                );
                let json = parse(&["notion", "search", "test", "--json"]);
                assert_eq!(
                    json.resolve_output_format(is_terminal, is_api),
                    OutputFormat::Json
                );
            }
        }
        let both = parse(&["notion", "search", "test", "--format", "plain", "--json"]);
        assert_eq!(both.resolve_output_format(true, false), OutputFormat::Json);
    }

    #[test]
    fn db_query_rejects_unimplemented_filter_dsl() {
        let error = Cli::try_parse_from([
            "notion",
            "db",
            "query",
            "source-id",
            "--filter",
            "Status = Done",
        ])
        .err()
        .expect("the unpublished filter DSL must be rejected");

        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    }

    #[test]
    fn db_query_accepts_raw_filter_json() {
        let cli = Cli::try_parse_from([
            "notion",
            "db",
            "query",
            "source-id",
            "--filter-json",
            r#"{"property":"Status","status":{"equals":"Done"}}"#,
        ])
        .expect("raw Notion filter JSON should remain supported");

        let Command::Db(db) = cli.command else {
            panic!("expected db command");
        };
        let db::DbSubcommand::Query { filter_json, .. } = db.command else {
            panic!("expected db query command");
        };
        assert_eq!(
            filter_json.as_deref(),
            Some(r#"{"property":"Status","status":{"equals":"Done"}}"#)
        );
    }
}
