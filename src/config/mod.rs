use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::io::Write;

use serde::{Deserialize, Serialize};

use crate::error::CliError;
use crate::output::OutputFormat;

const SERVICE_NAME: &str = "notion-cli";
const SECRET_SERVICE_NAME: &str = "notion-cli-oauth-secret";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_profile_name")]
    pub default_profile: String,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
    #[serde(default)]
    pub defaults: Defaults,
}

fn default_profile_name() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub token: Option<String>,
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_client_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default = "default_format")]
    pub output_format: OutputFormat,
    #[serde(default = "default_page_size")]
    pub page_size: u8,
}

fn default_format() -> OutputFormat {
    OutputFormat::Plain
}

fn default_page_size() -> u8 {
    50
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            output_format: OutputFormat::Plain,
            page_size: 50,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_profile: "default".to_string(),
            profiles: HashMap::new(),
            defaults: Defaults::default(),
        }
    }
}

impl Config {
    /// Returns the config file path: `$XDG_CONFIG_HOME/notion-cli/config.json`
    pub fn path() -> Result<PathBuf, CliError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| CliError::Config("Cannot determine config directory".into()))?;
        Ok(config_dir.join("notion-cli").join("config.json"))
    }

    /// Load config from disk, returning default if file doesn't exist.
    pub fn load() -> Result<Self, CliError> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Config::default());
        }
        let contents = fs::read_to_string(&path)?;
        let config: Config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Save config to disk, creating parent directories as needed.
    pub fn save(&self) -> Result<(), CliError> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;

            let temp_path = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&temp_path)?;
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
            fs::rename(temp_path, &path)?;
        }

        #[cfg(not(unix))]
        fs::write(&path, contents)?;

        Ok(())
    }

    /// Get the current active profile.
    pub fn active_profile(&self) -> Option<&Profile> {
        self.profiles.get(&self.default_profile)
    }

    /// Resolve the API token using the priority chain:
    /// 1. Explicit token (from --token flag)
    /// 2. NOTION_API_TOKEN env var
    /// 3. OS keyring
    /// 4. Config file
    pub fn resolve_token(
        &self,
        explicit_token: Option<&str>,
        profile_name: Option<&str>,
    ) -> Result<String, CliError> {
        // 1. Explicit token
        if let Some(t) = explicit_token {
            return Ok(t.to_string());
        }

        // 2. Env var
        if let Ok(t) = std::env::var("NOTION_API_TOKEN") {
            if !t.is_empty() {
                return Ok(t);
            }
        }

        let profile_key = profile_name.unwrap_or(&self.default_profile);

        // 3. OS keyring
        if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, profile_key) {
            if let Ok(token) = entry.get_password() {
                return Ok(token);
            }
        }

        // 4. Config file
        if let Some(profile) = self.profiles.get(profile_key) {
            if let Some(ref t) = profile.token {
                return Ok(t.clone());
            }
        }

        Err(CliError::NotAuthenticated)
    }

    /// Store a token in the OS keyring, falling back to config file.
    pub fn store_token(&mut self, profile_name: &str, token: &str) -> Result<(), CliError> {
        // Try keyring first
        if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, profile_name) {
            if entry.set_password(token).is_ok() {
                // Ensure profile exists; clear any plaintext token from config
                let profile = self
                    .profiles
                    .entry(profile_name.to_string())
                    .or_insert_with(|| Profile {
                        token: None,
                        workspace_id: None,
                        oauth_client_id: None,
                        oauth_client_secret: None,
                    });
                profile.token = None;
                return Ok(());
            }
        }

        // Fallback: store in config file
        eprintln!("Warning: OS keyring unavailable; storing token in the 0600 config file");
        let profile = self
            .profiles
            .entry(profile_name.to_string())
            .or_insert_with(|| Profile {
                token: None,
                workspace_id: None,
                oauth_client_id: None,
                oauth_client_secret: None,
            });
        profile.token = Some(token.to_string());
        Ok(())
    }

    /// Delete a token from keyring and config.
    pub fn delete_token(&mut self, profile_name: &str) -> Result<(), CliError> {
        if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, profile_name) {
            let _ = entry.delete_credential();
        }
        if let Some(profile) = self.profiles.get_mut(profile_name) {
            profile.token = None;
        }
        Ok(())
    }

    /// Store an OAuth client secret in the OS keyring, falling back to config file.
    pub fn store_secret(&mut self, profile_name: &str, secret: &str) -> Result<(), CliError> {
        if let Ok(entry) = keyring::Entry::new(SECRET_SERVICE_NAME, profile_name) {
            if entry.set_password(secret).is_ok() {
                // Clear plaintext from config
                if let Some(profile) = self.profiles.get_mut(profile_name) {
                    profile.oauth_client_secret = None;
                }
                return Ok(());
            }
        }

        // Fallback: store in config file
        eprintln!("Warning: OS keyring unavailable; storing OAuth secret in the 0600 config file");
        let profile = self
            .profiles
            .entry(profile_name.to_string())
            .or_insert_with(|| Profile {
                token: None,
                workspace_id: None,
                oauth_client_id: None,
                oauth_client_secret: None,
            });
        profile.oauth_client_secret = Some(secret.to_string());
        Ok(())
    }

    /// Resolve OAuth client secret from keyring.
    pub fn resolve_secret(&self, profile_name: Option<&str>) -> Result<String, CliError> {
        let profile_key = profile_name.unwrap_or(&self.default_profile);
        if let Ok(entry) = keyring::Entry::new(SECRET_SERVICE_NAME, profile_key) {
            if let Ok(secret) = entry.get_password() {
                return Ok(secret);
            }
        }
        Err(CliError::Config(
            "OAuth client secret not found in keyring".into(),
        ))
    }

    /// Get a config value by dot-separated key.
    pub fn get_value(&self, key: &str) -> Option<String> {
        match key {
            "default_profile" => Some(self.default_profile.clone()),
            "defaults.output_format" => Some(format!("{}", self.defaults.output_format)),
            "defaults.page_size" => Some(self.defaults.page_size.to_string()),
            k if k.starts_with("oauth.") => {
                let profile = self.active_profile()?;
                match k {
                    "oauth.client_id" => profile.oauth_client_id.clone(),
                    "oauth.client_secret" => profile
                        .oauth_client_secret
                        .as_ref()
                        .map(|_| "***".to_string()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Set a config value by dot-separated key.
    pub fn set_value(&mut self, key: &str, value: &str) -> Result<(), CliError> {
        match key {
            "default_profile" => {
                self.default_profile = value.to_string();
            }
            "defaults.output_format" => {
                let fmt: OutputFormat = value
                    .parse()
                    .map_err(|_| CliError::Config(format!("Invalid output format: {value}")))?;
                self.defaults.output_format = fmt;
            }
            "defaults.page_size" => {
                let size: u8 = value
                    .parse()
                    .map_err(|_| CliError::Config(format!("Invalid page size: {value}")))?;
                if size == 0 || size > 100 {
                    return Err(CliError::Config("Page size must be 1-100".into()));
                }
                self.defaults.page_size = size;
            }
            "oauth.client_id" => {
                let profile_key = self.default_profile.clone();
                let profile = self.profiles.entry(profile_key).or_insert_with(|| Profile {
                    token: None,
                    workspace_id: None,
                    oauth_client_id: None,
                    oauth_client_secret: None,
                });
                profile.oauth_client_id = Some(value.to_string());
            }
            "oauth.client_secret" => {
                let profile_key = self.default_profile.clone();
                self.store_secret(&profile_key, value)?;
            }
            _ => {
                return Err(CliError::Config(format!("Unknown config key: {key}")));
            }
        }
        Ok(())
    }
}
