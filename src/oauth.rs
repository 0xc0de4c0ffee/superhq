use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::db::Database;

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const SCOPES: &str = "openid profile email offline_access";

/// Tokens returned from the OAuth flow.
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub id_token: Option<String>,
}

/// Account info extracted from the id_token JWT.
#[derive(Clone)]
pub struct AccountInfo {
    pub email: Option<String>,
    pub plan: Option<String>,
}

/// Generate PKCE code_verifier and code_challenge (S256).
fn generate_pkce() -> (String, String) {
    let mut verifier_bytes = [0u8; 32];
    getrandom::getrandom(&mut verifier_bytes).expect("getrandom failed");
    let verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    (verifier, challenge)
}

/// Generate a random state parameter for CSRF protection.
fn generate_state() -> String {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Build the authorization URL for the browser.
fn build_auth_url(code_challenge: &str, state: &str) -> String {
    let scopes_encoded = SCOPES.replace(' ', "+");
    format!(
        "{AUTH_URL}?client_id={CLIENT_ID}\
         &redirect_uri={REDIRECT_URI}\
         &response_type=code\
         &scope={scopes_encoded}\
         &code_challenge={code_challenge}\
         &code_challenge_method=S256\
         &state={state}\
         &id_token_add_organizations=true\
         &codex_cli_simplified_flow=true\
         &originator=codex_cli_rs"
    )
}

/// Extract the authorization code and state from the HTTP callback request.
fn extract_code_and_state(request: &str) -> Result<(String, String)> {
    let first_line = request.lines().next().context("empty request")?;
    let path = first_line
        .split_whitespace()
        .nth(1)
        .context("no path in request")?;
    let query = path.split('?').nth(1).context("no query params")?;

    // Check for error response
    let mut error = None;
    let mut error_desc = None;
    let mut code = None;
    let mut state = None;

    for param in query.split('&') {
        if let Some(v) = param.strip_prefix("code=") {
            code = Some(v.to_string());
        } else if let Some(v) = param.strip_prefix("state=") {
            state = Some(v.to_string());
        } else if let Some(v) = param.strip_prefix("error=") {
            error = Some(v.to_string());
        } else if let Some(v) = param.strip_prefix("error_description=") {
            error_desc = Some(v.replace('+', " "));
        }
    }

    if let Some(err) = error {
        let desc = error_desc.unwrap_or_default();
        anyhow::bail!("OAuth error: {err}: {desc}");
    }

    let code = code.context("no code parameter in callback")?;
    let state = state.context("no state parameter in callback")?;
    Ok((code, state))
}

/// Exchange an authorization code for tokens.
async fn exchange_code(code: &str, verifier: &str) -> Result<OAuthTokens> {
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "authorization_code"),
        ("client_id", CLIENT_ID),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("code_verifier", verifier),
    ];

    let resp = client
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await
        .context("token exchange request failed")?;

    let body = resp
        .json::<serde_json::Value>()
        .await
        .context("failed to parse token response")?;

    if let Some(err) = body.get("error").and_then(|e| e.as_str()) {
        let desc = body
            .get("error_description")
            .and_then(|d| d.as_str())
            .unwrap_or("");
        anyhow::bail!("OAuth error: {err}: {desc}");
    }

    let access_token = body["access_token"]
        .as_str()
        .context("no access_token in response")?
        .to_string();

    Ok(OAuthTokens {
        access_token,
        refresh_token: body["refresh_token"].as_str().map(|s| s.to_string()),
        expires_in: body["expires_in"].as_u64(),
        id_token: body["id_token"].as_str().map(|s| s.to_string()),
    })
}

/// Refresh an OAuth access token using a refresh token.
pub async fn refresh_access_token(refresh_tok: &str) -> Result<OAuthTokens> {
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "refresh_token"),
        ("client_id", CLIENT_ID),
        ("refresh_token", refresh_tok),
    ];

    let resp = client
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await
        .context("token refresh request failed")?;

    let body = resp
        .json::<serde_json::Value>()
        .await
        .context("failed to parse refresh response")?;

    if let Some(err) = body.get("error").and_then(|e| e.as_str()) {
        anyhow::bail!("OAuth refresh error: {err}");
    }

    let access_token = body["access_token"]
        .as_str()
        .context("no access_token in refresh response")?
        .to_string();

    Ok(OAuthTokens {
        access_token,
        refresh_token: body["refresh_token"].as_str().map(|s| s.to_string()),
        expires_in: body["expires_in"].as_u64(),
        id_token: body["id_token"].as_str().map(|s| s.to_string()),
    })
}

/// Parse the id_token JWT to extract account info (email, plan).
/// Does NOT verify the signature — we trust the token from auth.openai.com.
pub fn parse_id_token(id_token: &str) -> Option<AccountInfo> {
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    // JWT payload is base64url-encoded, may need padding
    let payload = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&payload).ok()?;

    let email = json["email"].as_str().map(|s| s.to_string());

    let auth_claims = json.get("https://api.openai.com/auth");

    let plan = auth_claims
        .and_then(|v| v.get("user_tier"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            json.get("https://api.openai.com/profile")
                .and_then(|v| v.get("tier"))
                .and_then(|v| v.as_str())
        })
        .map(|s| s.to_string());

    Some(AccountInfo {
        email,
        plan,
    })
}

/// Run the full OAuth PKCE login flow: open browser, wait for callback, exchange code.
/// Pass a `cancel` receiver to allow the UI to abort the flow.
pub async fn login(cancel: tokio::sync::oneshot::Receiver<()>) -> Result<OAuthTokens> {
    let (verifier, challenge) = generate_pkce();
    let state = generate_state();
    let auth_url = build_auth_url(&challenge, &state);

    // Start callback server before opening browser
    let listener = tokio::net::TcpListener::bind("127.0.0.1:1455")
        .await
        .context("failed to bind callback server on port 1455")?;

    // Open browser
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg(&auth_url)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&auth_url)
            .spawn();
    }

    // Wait for callback with timeout, or cancellation from UI
    let accept_result = tokio::select! {
        result = tokio::time::timeout(
            std::time::Duration::from_secs(300),
            listener.accept(),
        ) => {
            result
                .context("login timed out after 5 minutes")?
                .context("failed to accept callback connection")
        }
        _ = cancel => {
            anyhow::bail!("Login cancelled")
        }
    };
    let (mut stream, _) = accept_result?;

    // Read HTTP request
    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await.context("reading callback")?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Extract authorization code and verify state
    let (code, returned_state) = extract_code_and_state(&request)?;
    if returned_state != state {
        anyhow::bail!("OAuth state mismatch — possible CSRF attack");
    }

    // Send success response to browser
    let response = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Content-Type: text/html\r\n",
        "Connection: close\r\n\r\n",
        "<html><body style='font-family:system-ui;text-align:center;padding:60px'>",
        "<h2>Signed in successfully</h2>",
        "<p style='color:#666'>You can close this tab and return to superhq.</p>",
        "</body></html>"
    );
    let _ = stream.write_all(response.as_bytes()).await;
    drop(stream);
    drop(listener);

    // Exchange code for tokens
    exchange_code(&code, &verifier).await
}

/// Save OAuth tokens to the database as the OPENAI_API_KEY secret.
pub fn save_openai_oauth(db: &Database, tokens: &OAuthTokens) -> Result<AccountInfo> {
    let expires_at = tokens.expires_in.map(|secs| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let ts = now + secs;
        format!("{ts}")
    });

    let hosts = vec![
        "api.openai.com".into(),
        "chatgpt.com".into(),
    ];

    db.save_oauth_secret(
        "OPENAI_API_KEY",
        "OpenAI (OAuth)",
        &tokens.access_token,
        tokens.refresh_token.as_deref(),
        tokens.id_token.as_deref(),
        expires_at.as_deref(),
        &hosts,
    )?;

    let account_info = tokens
        .id_token
        .as_deref()
        .and_then(parse_id_token)
        .unwrap_or(AccountInfo {
            email: None,
            plan: None,
        });

    Ok(account_info)
}

/// Check if an OAuth token needs refresh and refresh it if so.
/// Returns true if a refresh was performed.
pub async fn refresh_if_needed(db: &Arc<Database>, env_var: &str) -> Result<bool> {
    let auth_method = match db.get_secret_auth_method(env_var) {
        Ok(m) if m == "oauth" => m,
        _ => return Ok(false),
    };
    let _ = auth_method;

    let expires_at = match db.get_oauth_expires_at(env_var)? {
        Some(ea) => ea,
        None => return Ok(false),
    };

    // Parse expiry as unix timestamp
    let expires_ts: u64 = match expires_at.parse() {
        Ok(ts) => ts,
        Err(_) => return Ok(false),
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Refresh if within 5 minutes of expiry
    if now + 300 < expires_ts {
        return Ok(false);
    }

    let refresh_tok = match db.get_oauth_refresh_token(env_var)? {
        Some(rt) => rt,
        None => return Ok(false),
    };

    let tokens = refresh_access_token(&refresh_tok).await?;

    let expires_at = tokens.expires_in.map(|secs| format!("{}", now + secs));
    let hosts = vec![
        "api.openai.com".into(),
        "chatgpt.com".into(),
    ];

    // Preserve existing id_token if refresh didn't return a new one
    let existing_id_token = db.get_oauth_id_token(env_var)?;
    let id_token = tokens
        .id_token
        .as_deref()
        .or(existing_id_token.as_deref());

    db.save_oauth_secret(
        env_var,
        "OpenAI (OAuth)",
        &tokens.access_token,
        tokens.refresh_token.as_deref().or(Some(&refresh_tok)),
        id_token,
        expires_at.as_deref(),
        &hosts,
    )?;

    Ok(true)
}
