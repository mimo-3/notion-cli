use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::io::Write;

use serde::{Deserialize, Serialize};

use crate::error::CliError;
use crate::output::OutputFormat;

/// Secrets for all profiles, kept in a separate 0600 file so the main
/// config stays free of credentials. No `Debug` derive: tokens must never
/// reach logs or error output.
#[derive(Default, Serialize, Deserialize)]
pub struct CredentialsStore {
    #[serde(default)]
    tokens: HashMap<String, String>,
    #[serde(default)]
    oauth_secrets: HashMap<String, String>,
}

impl CredentialsStore {
    /// Returns the credentials file path: `credentials.json` in the platform
    /// config directory (`$XDG_CONFIG_HOME/notion-cli` on Linux,
    /// `~/Library/Application Support/notion-cli` on macOS).
    pub fn path() -> Result<PathBuf, CliError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| CliError::Config("Cannot determine config directory".into()))?;
        Ok(config_dir.join("notion-cli").join("credentials.json"))
    }

    /// Load credentials from disk, returning an empty store if the file doesn't exist.
    /// Warns on stderr if the file is readable by group or others.
    pub fn load() -> Result<Self, CliError> {
        let path = Self::path()?;
        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
        };

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&path) {
                if meta.permissions().mode() & 0o077 != 0 {
                    eprintln!(
                        "Warning: {} is readable by other users; run `chmod 600` on it",
                        path.display()
                    );
                }
            }
        }

        let store: Self = serde_json::from_str(&contents)?;
        Ok(store)
    }

    /// Save credentials to disk with owner-only permissions.
    pub fn save(&self) -> Result<(), CliError> {
        let path = Self::path()?;
        let contents = serde_json::to_string_pretty(self)?;
        write_private(&path, &contents)
    }
}

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

#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Legacy plaintext token slot; new logins keep this `None` and store
    /// the token in the credentials file instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_client_secret: Option<String>,
}

impl std::fmt::Debug for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Profile")
            .field("token", &self.token.as_ref().map(|_| "***"))
            .field("workspace_id", &self.workspace_id)
            .field("oauth_client_id", &self.oauth_client_id)
            .field(
                "oauth_client_secret",
                &self.oauth_client_secret.as_ref().map(|_| "***"),
            )
            .finish()
    }
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

/// Atomically write `contents` to `path` with owner-only permissions,
/// creating the parent directory (0700 on Unix) as needed.
fn write_private(path: &std::path::Path, contents: &str) -> Result<(), CliError> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
            }
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        let temp_path = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
        let result = (|| -> Result<(), CliError> {
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&temp_path)?;
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temp_path, path)?;
            Ok(())
        })();
        // Don't leave a plaintext temp file behind on failure
        if result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        result?;
    }

    #[cfg(not(unix))]
    fs::write(path, contents)?;

    Ok(())
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
        write_private(&path, &contents)
    }

    /// Get the current active profile.
    pub fn active_profile(&self) -> Option<&Profile> {
        self.profiles.get(&self.default_profile)
    }

    /// Resolve the API token using the priority chain:
    /// 1. Explicit token (from --token flag)
    /// 2. NOTION_API_TOKEN env var
    /// 3. Credentials file
    /// 4. Legacy plaintext token in the config file
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

        // 3. Credentials file
        if let Ok(store) = CredentialsStore::load() {
            if let Some(token) = store.tokens.get(profile_key) {
                return Ok(token.clone());
            }
        }

        // 4. Legacy config file (older versions stored the token here)
        if let Some(profile) = self.profiles.get(profile_key) {
            if let Some(ref t) = profile.token {
                return Ok(t.clone());
            }
        }

        Err(CliError::NotAuthenticated)
    }

    /// Store a token in the credentials file and clear any plaintext copy
    /// from the config.
    pub fn store_token(&mut self, profile_name: &str, token: &str) -> Result<(), CliError> {
        let mut store = CredentialsStore::load()?;
        store
            .tokens
            .insert(profile_name.to_string(), token.to_string());
        store.save()?;

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
        Ok(())
    }

    /// Delete a token and OAuth secret from the credentials file and config.
    pub fn delete_token(&mut self, profile_name: &str) -> Result<(), CliError> {
        let mut store = CredentialsStore::load()?;
        let removed_token = store.tokens.remove(profile_name).is_some();
        let removed_secret = store.oauth_secrets.remove(profile_name).is_some();
        if removed_token || removed_secret {
            store.save()?;
        }
        if let Some(profile) = self.profiles.get_mut(profile_name) {
            profile.token = None;
            profile.oauth_client_secret = None;
        }
        Ok(())
    }

    /// Store an OAuth client secret in the credentials file and clear any
    /// plaintext copy from the config.
    pub fn store_secret(&mut self, profile_name: &str, secret: &str) -> Result<(), CliError> {
        let mut store = CredentialsStore::load()?;
        store
            .oauth_secrets
            .insert(profile_name.to_string(), secret.to_string());
        store.save()?;

        if let Some(profile) = self.profiles.get_mut(profile_name) {
            profile.oauth_client_secret = None;
        }
        Ok(())
    }

    /// Resolve OAuth client secret from the credentials file, falling back
    /// to a legacy plaintext copy in the config file.
    pub fn resolve_secret(&self, profile_name: Option<&str>) -> Result<String, CliError> {
        let profile_key = profile_name.unwrap_or(&self.default_profile);
        let store = CredentialsStore::load()?;
        if let Some(secret) = store.oauth_secrets.get(profile_key) {
            return Ok(secret.clone());
        }
        if let Some(profile) = self.profiles.get(profile_key) {
            if let Some(ref s) = profile.oauth_client_secret {
                return Ok(s.clone());
            }
        }
        Err(CliError::Config(
            "OAuth client secret not found in the credentials file".into(),
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
