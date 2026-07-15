use clap::{Args, Subcommand};

use crate::config::Config;
use crate::error::CliError;

#[derive(Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Subcommand)]
pub enum ConfigSubcommand {
    /// Get a config value
    Get {
        /// Config key (e.g. defaults.output_format)
        key: String,
    },
    /// Set a config value
    Set {
        /// Config key
        key: String,
        /// Value to set
        value: String,
    },
    /// List all config values
    List,
    /// Show config file path
    Path,
}

pub async fn run(cmd: ConfigCommand, config: &mut Config) -> Result<(), CliError> {
    match cmd.command {
        ConfigSubcommand::Get { key } => {
            match config.get_value(&key) {
                Some(val) => println!("{val}"),
                None => eprintln!("Unknown key: {key}"),
            }
            Ok(())
        }
        ConfigSubcommand::Set { key, value } => {
            config.set_value(&key, &value)?;
            config.save()?;
            if key.contains("secret") || key.contains("token") {
                println!("{key} = ***");
            } else {
                println!("{key} = {value}");
            }
            Ok(())
        }
        ConfigSubcommand::List => {
            println!("default_profile = {}", config.default_profile);
            println!("defaults.output_format = {}", config.defaults.output_format);
            println!("defaults.page_size = {}", config.defaults.page_size);
            println!();
            println!("Profiles:");
            for name in config.profiles.keys() {
                let marker = if name == &config.default_profile {
                    " (active)"
                } else {
                    ""
                };
                println!("  {name}{marker}");
            }
            Ok(())
        }
        ConfigSubcommand::Path => {
            let path = Config::path()?;
            println!("{}", path.display());
            Ok(())
        }
    }
}
