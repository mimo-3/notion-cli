use clap::{Args, Subcommand};
use rand::Rng;

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
    /// Log in with a Notion API token or via browser OAuth
    Login {
        /// API token (prompted if not provided and --browser is not used)
        #[arg(long)]
        token: Option<String>,
        /// Profile name to store the token under
        #[arg(long, default_value = "default")]
        profile: String,
        /// Use browser-based OAuth login (requires --client-id and --client-secret, or set via config)
        #[arg(long)]
        browser: bool,
        /// OAuth client ID (can also be set via `notion config set oauth.client_id <id>`)
        #[arg(long)]
        client_id: Option<String>,
        /// OAuth client secret (can also be set via `notion config set oauth.client_secret <secret>`)
        #[arg(long)]
        client_secret: Option<String>,
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
        AuthSubcommand::Login {
            token,
            profile,
            browser,
            client_id,
            client_secret,
        } => {
            if browser {
                login_browser(client_id, client_secret, &profile, config).await
            } else {
                login(token, &profile, config, global).await
            }
        }
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

async fn login_browser(
    client_id: Option<String>,
    client_secret: Option<String>,
    profile: &str,
    config: &mut Config,
) -> Result<(), CliError> {
    // Resolve client_id and client_secret from flags or config
    let client_id = client_id
        .or_else(|| {
            config
                .profiles
                .get(profile)
                .and_then(|p| p.oauth_client_id.clone())
        })
        .ok_or_else(|| {
            CliError::OAuth(
                "Missing --client-id. Provide it as a flag or set it via: notion config set oauth.client_id <id>".into(),
            )
        })?;

    let client_secret = client_secret
        .or_else(|| config.resolve_secret(Some(profile)).ok())
        .or_else(|| {
            config
                .profiles
                .get(profile)
                .and_then(|p| p.oauth_client_secret.clone())
        })
        .map(Ok)
        .unwrap_or_else(|| {
            // Prompt for secret via stdin to avoid argv exposure
            dialoguer::Password::new()
                .with_prompt("Enter your OAuth client secret")
                .interact()
                .map_err(|e| CliError::OAuth(format!("Failed to read client secret: {e}")))
        })?;

    // Generate a CSPRNG state parameter for CSRF protection
    let state: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    // Start a local TCP listener on a random port
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| CliError::OAuth(format!("Failed to bind local server: {e}")))?;
    let port = listener
        .local_addr()
        .map_err(|e| CliError::OAuth(format!("Failed to get local address: {e}")))?
        .port();

    let redirect_uri = format!("http://localhost:{port}/callback");
    let auth_url = format!(
        "https://api.notion.com/v1/oauth/authorize?client_id={}&response_type=code&redirect_uri={}&owner=user&state={}",
        client_id,
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&state),
    );

    eprintln!("Opening browser for Notion authorization...");
    eprintln!("If the browser doesn't open, visit: {auth_url}");

    if let Err(e) = open::that(&auth_url) {
        eprintln!("Failed to open browser: {e}");
        eprintln!("Please open the URL above manually.");
    }

    eprintln!("Waiting for authorization callback on port {port}...");

    use std::io::{Read, Write};

    // Loop to accept connections, rejecting invalid ones (max 10 attempts)
    let mut code = String::new();
    let max_attempts = 10;
    let timeout = std::time::Duration::from_secs(300);
    let deadline = std::time::Instant::now() + timeout;

    for attempt in 0..max_attempts {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(CliError::OAuth(
                "Timed out waiting for authorization callback".into(),
            ));
        }

        // Use non-blocking + poll loop for accept timeout
        listener
            .set_nonblocking(true)
            .map_err(|e| CliError::OAuth(format!("Failed to configure listener: {e}")))?;

        let (mut stream, peer_addr) = loop {
            match listener.accept() {
                Ok(conn) => break conn,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        return Err(CliError::OAuth(
                            "Timed out waiting for authorization callback".into(),
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                Err(e) => {
                    return Err(CliError::OAuth(format!("Failed to accept connection: {e}")));
                }
            }
        };

        stream.set_nonblocking(false).ok();
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));

        // Only accept connections from localhost
        if !peer_addr.ip().is_loopback() {
            let _ = send_error_response(&mut stream, 403, "Forbidden");
            continue;
        }

        // Read the HTTP request
        let mut buf = [0u8; 4096];
        let n = match stream.read(&mut buf) {
            Ok(n) => n,
            Err(_) => {
                let _ = send_error_response(&mut stream, 400, "Bad Request");
                continue;
            }
        };
        let request = String::from_utf8_lossy(&buf[..n]);

        // Validate and extract code with state verification
        match extract_code_from_request(&request, &state) {
            Ok(extracted_code) => {
                // Send success response
                let response_body = "<html><body><h2>Authorization successful!</h2><p>You can close this tab and return to the terminal.</p></body></html>";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                let _ = stream.write_all(response.as_bytes());
                code = extracted_code;
                break;
            }
            Err(e) => {
                let authorization_denied = matches!(&e, CliError::OAuth(message) if message.starts_with("Authorization denied"));
                eprintln!(
                    "Rejected connection (attempt {}/{}): {e}",
                    attempt + 1,
                    max_attempts
                );
                let _ = send_error_response(&mut stream, 400, "Invalid callback request");
                if authorization_denied {
                    return Err(e);
                }
                continue;
            }
        }
    }

    if code.is_empty() {
        return Err(CliError::OAuth(
            "Failed to receive valid authorization callback after maximum attempts".into(),
        ));
    }

    eprintln!("Authorization code received. Exchanging for token...");

    // Exchange the code for an access token
    let access_token =
        exchange_code_for_token(&client_id, &client_secret, &code, &redirect_uri).await?;

    // Validate by calling /v1/users/me
    let client = NotionClient::new(access_token.clone())?;
    let user = client.get_self().await?;

    let name = user
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");

    // Store the token
    config.store_token(profile, &access_token)?;

    // Store OAuth credentials in the profile for future use
    let p = config
        .profiles
        .entry(profile.to_string())
        .or_insert_with(|| crate::config::Profile {
            token: None,
            workspace_id: None,
            oauth_client_id: None,
            oauth_client_secret: None,
        });
    p.oauth_client_id = Some(client_id);
    // Store client_secret in the credentials file; clear plaintext from config
    config.store_secret(profile, &client_secret)?;

    // Store workspace info if available
    if let Some(ws_name) = user
        .get("bot")
        .and_then(|b| b.get("workspace_name"))
        .and_then(|v| v.as_str())
    {
        if let Some(p) = config.profiles.get_mut(profile) {
            p.workspace_id = Some(ws_name.to_string());
        }
    }

    config.save()?;

    eprintln!("Logged in as {name} via OAuth on profile \"{profile}\"");
    Ok(())
}

/// Send an HTTP error response to the client.
fn send_error_response(
    stream: &mut impl std::io::Write,
    status: u16,
    message: &str,
) -> Result<(), std::io::Error> {
    let body = format!("<html><body><h2>{status} {message}</h2></body></html>");
    let response = format!(
        "HTTP/1.1 {status} {message}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())
}

/// Extract and validate the authorization code from the OAuth callback request.
///
/// Validates that the request is a GET to /callback with a matching state parameter.
fn extract_code_from_request(request: &str, expected_state: &str) -> Result<String, CliError> {
    // Parse the first line: GET /callback?code=...&state=... HTTP/1.1
    let first_line = request
        .lines()
        .next()
        .ok_or_else(|| CliError::OAuth("Empty request".into()))?;

    let mut parts = first_line.split_whitespace();

    // Validate HTTP method is GET
    let method = parts
        .next()
        .ok_or_else(|| CliError::OAuth("Malformed request".into()))?;
    if method != "GET" {
        return Err(CliError::OAuth(format!(
            "Expected GET method, got {method}"
        )));
    }

    let path = parts
        .next()
        .ok_or_else(|| CliError::OAuth("Malformed request".into()))?;

    // Validate path starts with /callback
    if !path.starts_with("/callback") {
        return Err(CliError::OAuth(format!("Unexpected path: {path}")));
    }

    // Parse query parameters
    let url = url::Url::parse(&format!("http://localhost{path}"))
        .map_err(|e| CliError::OAuth(format!("Failed to parse callback URL: {e}")))?;

    // Validate state parameter matches
    let received_state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| CliError::OAuth("Missing state parameter in callback".into()))?;

    if received_state != expected_state {
        return Err(CliError::OAuth(
            "State parameter mismatch — possible CSRF attack".into(),
        ));
    }

    if let Some(error) = url
        .query_pairs()
        .find(|(key, _)| key == "error")
        .map(|(_, value)| value.to_string())
    {
        return Err(CliError::OAuth(format!(
            "Authorization denied or failed: {error}"
        )));
    }

    // Extract code parameter
    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| CliError::OAuth("No authorization code in callback".into()))?;

    Ok(code)
}

async fn exchange_code_for_token(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<String, CliError> {
    use base64::Engine;

    let credentials =
        base64::engine::general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));

    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| CliError::OAuth(format!("Failed to build token exchange client: {e}")))?;
    let resp = http
        .post("https://api.notion.com/v1/oauth/token")
        .header("Authorization", format!("Basic {credentials}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": redirect_uri,
        }))
        .send()
        .await
        .map_err(|e| CliError::OAuth(format!("Token exchange request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(CliError::OAuth(format!(
            "Token exchange failed ({status}): {body}"
        )));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| CliError::OAuth(format!("Failed to parse token response: {e}")))?;

    let access_token = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::OAuth("No access_token in response".into()))?
        .to_string();

    Ok(access_token)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_denial_with_matching_state_is_terminal() {
        let result = extract_code_from_request(
            "GET /callback?error=access_denied&state=expected HTTP/1.1\r\n\r\n",
            "expected",
        );

        assert!(matches!(
            result,
            Err(CliError::OAuth(message)) if message.starts_with("Authorization denied")
        ));
    }

    #[test]
    fn oauth_denial_cannot_bypass_state_validation() {
        let result = extract_code_from_request(
            "GET /callback?error=access_denied&state=wrong HTTP/1.1\r\n\r\n",
            "expected",
        );

        assert!(matches!(
            result,
            Err(CliError::OAuth(message)) if message.contains("State parameter mismatch")
        ));
    }
}
