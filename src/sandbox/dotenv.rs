//! Parse the root `.env` file from a host directory before mounting into the sandbox.
//!
//! Instead of mounting `.env` directly (exposing plaintext secrets),
//! we parse it on the host side and route values through the secrets proxy.
//!
//! TODO: At workspace creation time, detect all .env files in the directory
//! and let the user pick which ones to include (e.g. .env, .env.local, .env.production).

use std::collections::HashMap;
use std::path::Path;

/// Parse the root `.env` file in `dir`, returning key-value pairs.
pub fn parse_env(dir: &Path) -> HashMap<String, String> {
    let path = dir.join(".env");
    let mut vars = HashMap::new();
    if path.is_file() {
        if let Ok(iter) = dotenvy::from_path_iter(&path) {
            for item in iter.flatten() {
                vars.insert(item.0, item.1);
            }
        }
    }
    vars
}

/// Returns the guest path for `.env` if it exists in `dir`.
pub fn env_guest_path(dir: &Path) -> Option<&'static str> {
    if dir.join(".env").is_file() {
        Some("/workspace/.env")
    } else {
        None
    }
}
