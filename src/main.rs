mod api;
mod cli;
mod client;
mod config;
mod error;
mod filter;
mod models;
mod output;

use std::process;

use clap::Parser;

use cli::{Cli, Command};
use config::Config;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.global.no_color {
        colored::control::set_override(false);
    }

    let result = run(cli).await;
    match result {
        Ok(()) => {}
        Err(e) => {
            let code = e.exit_code();
            eprintln!("Error: {e}");
            process::exit(code);
        }
    }
}

/// Extract a Notion ID from a URL or normalize a raw ID.
pub fn normalize_id(id_or_url: &str) -> String {
    if id_or_url.contains("notion.so") || id_or_url.contains("notion.site") {
        if let Some(last) = id_or_url.split('/').last() {
            let cleaned = last.split('?').next().unwrap_or(last);
            if let Some(id_part) = cleaned.split('-').last() {
                if id_part.len() == 32 && id_part.chars().all(|c| c.is_ascii_hexdigit()) {
                    return format!(
                        "{}-{}-{}-{}-{}",
                        &id_part[..8],
                        &id_part[8..12],
                        &id_part[12..16],
                        &id_part[16..20],
                        &id_part[20..]
                    );
                }
            }
        }
    }
    id_or_url.to_string()
}

async fn run(cli: Cli) -> Result<(), error::CliError> {
    let mut config = Config::load()?;

    match cli.command {
        Command::Auth(cmd) => cli::auth::run(cmd, &mut config, &cli.global).await,
        Command::Search(args) => {
            let client = client::NotionClient::from_opts(&cli.global, &config)?;
            cli::search::run(args, &client, &cli.global).await
        }
        Command::Page(cmd) => {
            let client = client::NotionClient::from_opts(&cli.global, &config)?;
            cli::page::run(cmd, &client, &cli.global).await
        }
        Command::Db(cmd) => {
            let client = client::NotionClient::from_opts(&cli.global, &config)?;
            cli::db::run(cmd, &client, &cli.global).await
        }
        Command::Block(cmd) => {
            let client = client::NotionClient::from_opts(&cli.global, &config)?;
            cli::block::run(cmd, &client, &cli.global).await
        }
        Command::Comment(cmd) => {
            let client = client::NotionClient::from_opts(&cli.global, &config)?;
            cli::comment::run(cmd, &client, &cli.global).await
        }
        Command::User(cmd) => {
            let client = client::NotionClient::from_opts(&cli.global, &config)?;
            cli::user::run(cmd, &client, &cli.global).await
        }
        Command::File(cmd) => {
            let client = client::NotionClient::from_opts(&cli.global, &config)?;
            cli::file::run(cmd, &client, &cli.global).await
        }
        Command::View(_cmd) => {
            eprintln!("View commands are not yet implemented (Phase 2).");
            Ok(())
        }
        Command::Api(args) => {
            let client = client::NotionClient::from_opts(&cli.global, &config)?;
            cli::api::run(args, &client, &cli.global).await
        }
        Command::Config(cmd) => cli::config_cmd::run(cmd, &mut config).await,
    }
}
