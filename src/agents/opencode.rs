use super::{secret_entry, AgentConfig, AuthGatewaySpec, InstallStep, NODE_INSTALL_STEP};
use crate::db::{Database, RequiredSecretEntry};
use shuru_sdk::{AsyncSandbox, MountConfig};
use std::collections::HashMap;
use std::path::PathBuf;

/// opencode never uses superhq's auth gateway.
///
/// The gateway in OAuth mode rewrites api.openai.com → chatgpt.com/backend-api/codex,
/// which is Codex-CLI-specific: it expects an `instructions` field and the custom
/// payload shape that Codex and Pi's `openai-codex-responses` API type produce.
/// opencode uses the standard AI SDK `openai` provider and sends the plain
/// OpenAI Responses payload, so the Codex backend rejects its requests (400
/// "Instructions are required").
///
/// OpenAI OAuth still works with opencode because it uses the same ChatGPT
/// OAuth client_id and backend as Codex / superhq — we just hand the tokens
/// to opencode directly instead of proxying. See [`sync_auth_json`] below,
/// which seeds opencode's `auth.json` from the superhq vault on each boot.
pub fn auth_gateway_spec(_db: &Database) -> Option<AuthGatewaySpec> {
    None
}

/// Drop `OPENAI_API_KEY` from the MITM-proxy secrets list when it's stored
/// as OAuth. Those tokens are wired into opencode via [`sync_auth_json`]
/// (as an `auth.json` `"oauth"` entry) — if we also injected them as an
/// env var, opencode would see `OPENAI_API_KEY` and select its plain
/// api-key path, sending requests to the wrong endpoint with the wrong
/// token shape.
///
/// API-key mode for OpenAI is kept — it flows through the MITM proxy like
/// Anthropic and OpenRouter keys.
pub fn filter_required_secrets(
    db: &Database,
    required: Vec<RequiredSecretEntry>,
) -> Vec<RequiredSecretEntry> {
    required
        .into_iter()
        .filter(|entry| {
            if entry.env_var() != "OPENAI_API_KEY" {
                return true;
            }
            let method = db
                .get_secret_auth_method("OPENAI_API_KEY")
                .unwrap_or_else(|_| "api_key".into());
            method != "oauth"
        })
        .collect()
}

pub fn config() -> AgentConfig {
    AgentConfig {
        name: "opencode",
        display_name: "opencode",
        command: "/usr/local/bin/opencode",
        icon: Some("icons/agents/opencode.svg"),
        color: Some("#7C6AF6"),
        tab_order: 2,
        install_steps: vec![
            NODE_INSTALL_STEP,
            InstallStep::Cmd {
                label: "Installing opencode",
                command: "/usr/local/bin/npm install -g opencode-ai",
                skip_if: Some("/usr/local/bin/opencode --version"),
            },
            InstallStep::Cmd {
                label: "Verifying installation",
                command: "/usr/local/bin/opencode --version",
                skip_if: None,
            },
        ],
        // All three providers optional — opencode auto-configures whichever
        // subset the user has. When nothing is configured, users can still
        // launch opencode and add credentials via /connect (which persist
        // via the extra_mounts() host-side directory).
        secrets: vec![
            secret_entry(
                "ANTHROPIC_API_KEY",
                "Anthropic API Key",
                &["api.anthropic.com"],
                &[],
                true,
            ),
            // Hosts left empty → falls back to default_hosts() (api.openai.com
            // + chatgpt.com) for the MITM proxy. OAuth entries are filtered
            // out at boot by filter_required_secrets() since they can't drive
            // opencode's standard openai provider.
            secret_entry("OPENAI_API_KEY", "OpenAI API Key", &[], &[], true),
            secret_entry(
                "OPENROUTER_API_KEY",
                "OpenRouter API Key",
                &["openrouter.ai"],
                &[],
                true,
            ),
        ],
        // Dynamic — resolved at boot via auth_gateway_spec() above.
        auth_gateway: None,
    }
}

/// No post-boot config writes needed: opencode auto-detects standard env
/// vars (ANTHROPIC_API_KEY / OPENAI_API_KEY / OPENROUTER_API_KEY) that the
/// MITM proxy injects as placeholders. Additional providers go through
/// opencode's own `/connect` flow, persisted via [`extra_mounts`].
pub async fn auth_setup(_sandbox: &AsyncSandbox, _vars: &HashMap<String, String>) {}

/// Mount a host-managed directory at opencode's auth path so credentials
/// added inside the sandbox via `/connect` persist across sessions /
/// fresh sandboxes. The host path is superhq-managed (not user workspace),
/// and opencode owns the file layout inside it (currently just auth.json).
pub fn extra_mounts() -> Vec<MountConfig> {
    let host_path = match auth_dir() {
        Some(p) => p,
        None => return Vec::new(),
    };
    if let Err(e) = std::fs::create_dir_all(&host_path) {
        eprintln!("[opencode] failed to create auth dir {}: {e}", host_path.display());
        return Vec::new();
    }
    vec![MountConfig {
        host_path: host_path.to_string_lossy().into_owned(),
        guest_path: "/root/.local/share/opencode".to_string(),
        read_only: false,
    }]
}

/// Host-side path of opencode's auth directory (mirrored into the sandbox
/// via [`extra_mounts`]). Returns `None` if `$HOME` isn't set.
fn auth_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".local/share/superhq/opencode-auth"))
}

/// Seed opencode's `auth.json` with superhq-managed OpenAI OAuth credentials
/// so opencode's own ChatGPT provider can use them directly (same OAuth
/// client_id and backend as Codex CLI / superhq).
///
/// This runs on every boot: it merges our OpenAI entry into whatever
/// `/connect` wrote in prior sessions, rather than overwriting. If OpenAI
/// isn't stored as OAuth (or isn't stored at all), any stale entry we
/// previously wrote is removed so we don't leave revoked tokens behind.
///
/// Called from the boot flow after `refresh_oauth_tokens` so `access` /
/// `expires` reflect a freshly refreshed token.
pub fn sync_auth_json(db: &Database) {
    let dir = match auth_dir() {
        Some(p) => p,
        None => return,
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("[opencode] failed to create auth dir {}: {e}", dir.display());
        return;
    }
    let auth_path = dir.join("auth.json");

    // Load existing auth.json (preserve /connect-added providers). Treat any
    // parse error as "start fresh" rather than clobbering — at worst we lose
    // one /connect entry, not the refresh tokens opencode needs to recover.
    let mut root: serde_json::Map<String, serde_json::Value> = match std::fs::read(&auth_path) {
        Ok(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::Map::new(),
        Err(e) => {
            eprintln!("[opencode] failed to read {}: {e}", auth_path.display());
            return;
        }
    };

    match build_openai_oauth_entry(db) {
        Some(entry) => {
            root.insert("openai".to_string(), entry);
        }
        None => {
            // Remove any entry we (or a prior boot) wrote for OpenAI via
            // OAuth. Leave user-added api_key entries alone — opencode's
            // /connect stores those with `"type": "api"`, distinct from our
            // `"type": "oauth"`, so we only strip our own flavor.
            let drop = matches!(
                root.get("openai").and_then(|v| v.get("type")).and_then(|v| v.as_str()),
                Some("oauth")
            );
            if drop {
                root.remove("openai");
            }
        }
    }

    let rendered = match serde_json::to_vec_pretty(&serde_json::Value::Object(root)) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[opencode] failed to serialize auth.json: {e}");
            return;
        }
    };

    if let Err(e) = write_secret_file(&auth_path, &rendered) {
        eprintln!("[opencode] failed to write {}: {e}", auth_path.display());
    }
}

/// Build the opencode `auth.json` entry for OpenAI when we have an OAuth
/// credential in the vault. Returns `None` for api_key mode (opencode
/// auto-reads `OPENAI_API_KEY` from env) or when tokens are missing.
///
/// Field mapping from superhq's DB to opencode's schema:
///   access_token     → access
///   refresh_token    → refresh
///   expires_at (s)   → expires (ms since epoch)
///   id_token.claim   → accountId  (chatgpt_account_id under the
///                                  `https://api.openai.com/auth` object)
fn build_openai_oauth_entry(db: &Database) -> Option<serde_json::Value> {
    let method = db.get_secret_auth_method("OPENAI_API_KEY").ok()?;
    if method != "oauth" {
        return None;
    }
    let access = db.get_secret_value("OPENAI_API_KEY").ok().flatten()?;
    let refresh = db.get_oauth_refresh_token("OPENAI_API_KEY").ok().flatten()?;
    let id_token = db.get_oauth_id_token("OPENAI_API_KEY").ok().flatten()?;
    let account_id =
        crate::sandbox::auth_gateway::extract_jwt_claim(&id_token, "chatgpt_account_id")?;

    // DB stores expires_at as a Unix-seconds string; opencode expects ms.
    let expires_ms: i64 = db
        .get_oauth_expires_at("OPENAI_API_KEY")
        .ok()
        .flatten()
        .and_then(|s| s.parse::<i64>().ok())
        .map(|secs| secs.saturating_mul(1000))
        .unwrap_or(0);

    Some(serde_json::json!({
        "type": "oauth",
        "access": access,
        "refresh": refresh,
        "expires": expires_ms,
        "accountId": account_id,
    }))
}

/// Write a file with 0600 permissions (auth.json holds OAuth tokens).
fn write_secret_file(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(bytes)?;
    // Ensure mode is tightened even if the file pre-existed with looser perms.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}
