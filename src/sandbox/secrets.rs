use crate::db::RequiredSecretEntry;
use crate::db::Database;
use crate::oauth;
use crate::sandbox::provider_resolve::{agent_need, load_provider_states, resolve};
use anyhow::Result;
use shuru_sdk::SecretConfig;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Default host patterns for well-known API key env vars.
/// Used as fallback when the RequiredSecret doesn't specify hosts
/// and the DB secret has no hosts stored.
pub fn default_hosts(env_var: &str) -> Vec<String> {
    match env_var {
        "ANTHROPIC_API_KEY" => vec!["api.anthropic.com".into()],
        "OPENAI_API_KEY" => vec![
            "api.openai.com".into(),
            "chatgpt.com".into(),
        ],
        "OPENROUTER_API_KEY" => vec!["openrouter.ai".into()],
        _ => vec![],
    }
}

/// Check which required secrets are missing from the vault.
/// A secret that's saved but disabled in settings counts as missing — the user
/// has explicitly opted out of it for this provider.
pub fn check_missing(db: &Database, required: &[RequiredSecretEntry]) -> Vec<RequiredSecretEntry> {
    let states = load_provider_states(db, required);
    let no_gateway = HashSet::new();
    let need = agent_need(required, &no_gateway);
    let r = resolve(&states, &need);

    let missing_set: HashSet<&str> = r.missing_required.iter().map(|s| s.as_str()).collect();
    let mut missing: Vec<RequiredSecretEntry> = required
        .iter()
        .filter(|e| missing_set.contains(e.env_var()))
        .cloned()
        .collect();

    if r.missing_one_of {
        missing.extend(required.iter().filter(|e| e.is_optional()).cloned());
    }

    missing
}

pub struct ResolvedSecrets {
    pub secrets: HashMap<String, SecretConfig>,
}

/// Build the secrets map for `SandboxConfig.secrets`.
/// Decrypts each required secret and constructs `SecretConfig` with direct values.
///
/// `gateway_env_vars` contains env var names handled by an auth gateway — these
/// are skipped for both MITM proxy setup and OAuth token injection (the gateway
/// handles auth on the host side).
pub fn build_secrets_map(
    db: &Database,
    required: &[RequiredSecretEntry],
    gateway_env_vars: &HashSet<&str>,
) -> Result<ResolvedSecrets> {
    let states = load_provider_states(db, required);
    let need = agent_need(required, gateway_env_vars);
    let inject = resolve(&states, &need).inject;
    let entries_by_env: HashMap<&str, &RequiredSecretEntry> =
        required.iter().map(|e| (e.env_var(), e)).collect();

    let mut secrets = HashMap::new();
    for env_var in inject {
        let Some(entry) = entries_by_env.get(env_var.as_str()) else { continue };
        let full = entry.as_full();

        if let Some(value) = db.get_secret_value(&env_var)? {
            // Resolve hosts: RequiredSecret.hosts > DB hosts > default_hosts
            let hosts: Vec<String> = if let Some(h) = full.hosts {
                h
            } else {
                let db_hosts = db.get_secret_hosts(&env_var)?;
                if db_hosts.is_empty() {
                    default_hosts(&env_var)
                } else {
                    db_hosts
                }
            };

            secrets.insert(
                env_var.clone(),
                SecretConfig {
                    from: env_var,
                    hosts,
                    value: Some(value),
                },
            );
        }
    }

    Ok(ResolvedSecrets { secrets })
}

/// Refresh any OAuth tokens that are near expiry. Call this before building the secrets map.
pub async fn refresh_oauth_tokens(db: &Arc<Database>, required: &[RequiredSecretEntry]) -> Result<()> {
    for entry in required {
        let env_var: &str = entry.env_var();
        if let Err(e) = oauth::refresh_if_needed(db, env_var).await {
            eprintln!("OAuth token refresh failed for {env_var}: {e}");
        }
    }
    Ok(())
}
