use clap::{Args, Subcommand};

use crate::cli::GlobalOpts;
use crate::client::NotionClient;
use crate::config::Config;
use crate::error::CliError;
use crate::output;

#[derive(Args)]
pub struct AuthCommand {
    #[command(subcommand)]
    pub command: AuthSubcommand,
}

#[derive(Subcommand)]
pub enum AuthSubcommand {
    /// Log in with a Notion API token
    Login {
        /// API token (prompted if not provided)
        #[arg(long)]
        token: Option<String>,
        /// Profile name to store the token under
        #[arg(long, default_value = "default")]
        profile: String,
    },
    /// Log out and remove stored credentials
    Logout {
        /// Profile to log out from
        #[arg(long, default_value = "default")]
        profile: String,
    },
    /// Show the currently authenticated user
    Whoami,
    /// Switch the default profile
    Switch {
        /// Profile name to switch to
        profile: String,
    },
}

pub async fn run(
    cmd: AuthCommand,
    config: &mut Config,
    global: &GlobalOpts,
) -> Result<(), CliError> {
    match cmd.command {
        AuthSubcommand::Login { token, profile } => login(token, &profile, config, global).await,
        AuthSubcommand::Logout { profile } => logout(&profile, config).await,
        AuthSubcommand::Whoami => whoami(config, global).await,
        AuthSubcommand::Switch { profile } => switch(&profile, config).await,
    }
}

async fn login(
    token: Option<String>,
    profile: &str,
    config: &mut Config,
    _global: &GlobalOpts,
) -> Result<(), CliError> {
    let token = match token {
        Some(t) => t,
        None => {
            // Prompt for token
            dialoguer::Password::new()
                .with_prompt("Enter your Notion API token")
                .interact()
                .map_err(|e| CliError::Config(format!("Failed to read token: {e}")))?
        }
    };

    // Validate the token by calling /v1/users/me
    eprintln!("Validating token...");
    let client = NotionClient::new(token.clone())?;
    let user = client.get_self().await?;

    let name = user
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let user_type = user
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Store token
    config.store_token(profile, &token)?;

    // Store workspace info if available
    if let Some(ws_id) = user
        .get("bot")
        .and_then(|b| b.get("workspace_name"))
        .and_then(|v| v.as_str())
    {
        if let Some(p) = config.profiles.get_mut(profile) {
            p.workspace_id = Some(ws_id.to_string());
        }
    }

    config.save()?;

    eprintln!("Logged in as {name} ({user_type}) on profile \"{profile}\"");
    Ok(())
}

async fn logout(profile: &str, config: &mut Config) -> Result<(), CliError> {
    config.delete_token(profile)?;
    config.save()?;
    eprintln!("Logged out from profile \"{profile}\"");
    Ok(())
}

async fn whoami(config: &Config, global: &GlobalOpts) -> Result<(), CliError> {
    let client = NotionClient::from_opts(global, config)?;
    let user = client.get_self().await?;

    let format = global.output_format();
    let mut stdout = std::io::stdout();
    output::format_value(&user, format, &mut stdout)?;
    Ok(())
}

async fn switch(profile: &str, config: &mut Config) -> Result<(), CliError> {
    if !config.profiles.contains_key(profile) {
        return Err(CliError::Config(format!(
            "Profile \"{profile}\" does not exist. Available profiles: {}",
            config
                .profiles
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    config.default_profile = profile.to_string();
    config.save()?;
    eprintln!("Switched to profile \"{profile}\"");
    Ok(())
}
