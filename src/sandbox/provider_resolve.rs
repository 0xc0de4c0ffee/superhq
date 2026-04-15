//! Pure provider-resolution logic.
//!
//! Given the state of every provider the user knows about and what an agent
//! needs, decides which env vars to inject into the sandbox and which (if any)
//! are missing from the user's perspective.
//!
//! No DB access, no IO. Construct [`ProviderState`]s from the DB at the call
//! site (see [`load_provider_states`]) and pass them in.

use crate::db::{Database, RequiredSecretEntry};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    ApiKey,
    OAuth,
}

impl AuthMethod {
    pub fn from_str(s: &str) -> Self {
        match s {
            "oauth" => Self::OAuth,
            _ => Self::ApiKey,
        }
    }
}

/// Snapshot of one provider's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderState {
    pub env_var: String,
    pub has_value: bool,
    pub enabled: bool,
    pub auth_method: AuthMethod,
}

impl ProviderState {
    /// A provider is "usable" if it's saved and the user hasn't disabled it.
    pub fn is_usable(&self) -> bool {
        self.has_value && self.enabled
    }
}

/// What an agent asks the sandbox to provide.
#[derive(Debug, Clone)]
pub struct AgentNeed<'a> {
    /// Env vars that must each be usable.
    pub required: Vec<&'a str>,
    /// Optional group: at least one member must be usable.
    pub one_of: Vec<&'a str>,
    /// Env vars whose auth flows through a host-side gateway — they should
    /// not be injected as plain values, even if usable.
    pub gateway: &'a HashSet<&'a str>,
}

/// The decision: what to inject and what's missing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resolution {
    /// Env vars to inject into the sandbox (usable AND not gateway-handled).
    pub inject: Vec<String>,
    /// Required env vars that aren't usable. Empty == OK.
    pub missing_required: Vec<String>,
    /// True iff the `one_of` group is non-empty and no member is usable.
    pub missing_one_of: bool,
}

pub fn resolve(states: &[ProviderState], need: &AgentNeed) -> Resolution {
    let usable = |env_var: &str| {
        states
            .iter()
            .find(|s| s.env_var == env_var)
            .is_some_and(|s| s.is_usable())
    };

    let missing_required: Vec<String> = need
        .required
        .iter()
        .filter(|v| !usable(v))
        .map(|v| (*v).to_string())
        .collect();

    let missing_one_of = !need.one_of.is_empty() && !need.one_of.iter().any(|v| usable(v));

    let mut inject: Vec<String> = states
        .iter()
        .filter(|s| s.is_usable() && !need.gateway.contains(s.env_var.as_str()))
        .map(|s| s.env_var.clone())
        .collect();
    inject.sort();

    Resolution {
        inject,
        missing_required,
        missing_one_of,
    }
}

// ── DB adapters (impure — kept here so callers don't have to assemble states by hand) ──

/// Load provider state for the env vars referenced by `entries`. Env vars with
/// no saved secret get `has_value = false`.
pub fn load_provider_states(db: &Database, entries: &[RequiredSecretEntry]) -> Vec<ProviderState> {
    let saved = db.list_secrets().unwrap_or_default();
    entries
        .iter()
        .map(|e| {
            let env_var = e.env_var();
            let row = saved.iter().find(|s| s.env_var == env_var);
            ProviderState {
                env_var: env_var.to_string(),
                has_value: row.is_some(),
                enabled: row.map(|r| r.enabled).unwrap_or(false),
                auth_method: row
                    .map(|r| AuthMethod::from_str(&r.auth_method))
                    .unwrap_or(AuthMethod::ApiKey),
            }
        })
        .collect()
}

/// Split a list of `RequiredSecretEntry` into an [`AgentNeed`].
pub fn agent_need<'a>(
    entries: &'a [RequiredSecretEntry],
    gateway_env_vars: &'a HashSet<&'a str>,
) -> AgentNeed<'a> {
    let mut required = Vec::new();
    let mut one_of = Vec::new();
    for e in entries {
        if e.is_optional() {
            one_of.push(e.env_var());
        } else {
            required.push(e.env_var());
        }
    }
    AgentNeed {
        required,
        one_of,
        gateway: gateway_env_vars,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(env_var: &str, has_value: bool, enabled: bool, auth: AuthMethod) -> ProviderState {
        ProviderState {
            env_var: env_var.into(),
            has_value,
            enabled,
            auth_method: auth,
        }
    }

    fn need<'a>(
        required: &'a [&'a str],
        one_of: &'a [&'a str],
        gateway: &'a HashSet<&'a str>,
    ) -> AgentNeed<'a> {
        AgentNeed {
            required: required.to_vec(),
            one_of: one_of.to_vec(),
            gateway,
        }
    }

    #[test]
    fn happy_path_required_only() {
        let states = vec![
            st("ANTHROPIC_API_KEY", true, true, AuthMethod::ApiKey),
            st("OPENAI_API_KEY", true, true, AuthMethod::ApiKey),
        ];
        let gw = HashSet::new();
        let need = need(&["ANTHROPIC_API_KEY"], &[], &gw);
        let r = resolve(&states, &need);
        // Anthropic was required; both saved+enabled providers get injected.
        assert_eq!(r.missing_required, Vec::<String>::new());
        assert!(!r.missing_one_of);
        assert_eq!(r.inject, vec!["ANTHROPIC_API_KEY", "OPENAI_API_KEY"]);
    }

    #[test]
    fn required_missing_when_disabled() {
        let states = vec![st("ANTHROPIC_API_KEY", true, false, AuthMethod::ApiKey)];
        let gw = HashSet::new();
        let need = need(&["ANTHROPIC_API_KEY"], &[], &gw);
        let r = resolve(&states, &need);
        assert_eq!(r.missing_required, vec!["ANTHROPIC_API_KEY"]);
        assert!(r.inject.is_empty());
    }

    #[test]
    fn required_missing_when_no_value() {
        let states = vec![st("ANTHROPIC_API_KEY", false, true, AuthMethod::ApiKey)];
        let gw = HashSet::new();
        let need = need(&["ANTHROPIC_API_KEY"], &[], &gw);
        let r = resolve(&states, &need);
        assert_eq!(r.missing_required, vec!["ANTHROPIC_API_KEY"]);
        assert!(r.inject.is_empty());
    }

    #[test]
    fn one_of_satisfied_by_any_usable_member() {
        let states = vec![
            st("OPENAI_API_KEY", false, false, AuthMethod::ApiKey),
            st("OPENROUTER_API_KEY", true, true, AuthMethod::ApiKey),
        ];
        let gw = HashSet::new();
        let need = need(&[], &["OPENAI_API_KEY", "OPENROUTER_API_KEY"], &gw);
        let r = resolve(&states, &need);
        assert!(!r.missing_one_of);
        assert_eq!(r.inject, vec!["OPENROUTER_API_KEY"]);
    }

    #[test]
    fn one_of_unsatisfied_when_all_disabled() {
        let states = vec![
            st("OPENAI_API_KEY", true, false, AuthMethod::OAuth),
            st("OPENROUTER_API_KEY", true, false, AuthMethod::ApiKey),
        ];
        let gw = HashSet::new();
        let need = need(&[], &["OPENAI_API_KEY", "OPENROUTER_API_KEY"], &gw);
        let r = resolve(&states, &need);
        assert!(r.missing_one_of);
        assert!(r.inject.is_empty());
    }

    #[test]
    fn empty_one_of_is_not_missing() {
        let gw = HashSet::new();
        let r = resolve(&[], &need(&[], &[], &gw));
        assert!(!r.missing_one_of);
    }

    #[test]
    fn gateway_env_vars_excluded_from_inject_but_count_for_required() {
        let states = vec![st("OPENAI_API_KEY", true, true, AuthMethod::OAuth)];
        let mut gw = HashSet::new();
        gw.insert("OPENAI_API_KEY");
        let need = AgentNeed {
            required: vec!["OPENAI_API_KEY"],
            one_of: vec![],
            gateway: &gw,
        };
        let r = resolve(&states, &need);
        // Required is satisfied (the secret exists & is enabled),
        // but inject is empty because the gateway handles it.
        assert_eq!(r.missing_required, Vec::<String>::new());
        assert!(r.inject.is_empty());
    }

    #[test]
    fn disabled_one_of_member_doesnt_satisfy_even_if_others_are_usable_outside_group() {
        let states = vec![
            st("OPENAI_API_KEY", true, false, AuthMethod::ApiKey),
            st("ANTHROPIC_API_KEY", true, true, AuthMethod::ApiKey),
        ];
        let gw = HashSet::new();
        let need = need(&[], &["OPENAI_API_KEY"], &gw);
        let r = resolve(&states, &need);
        assert!(r.missing_one_of);
        // Anthropic is still injected though — it's usable and not in any need.
        assert_eq!(r.inject, vec!["ANTHROPIC_API_KEY"]);
    }
}
