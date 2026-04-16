//! Fire-and-forget cache of GitHub owner avatars on disk. Lives under
//! `<data_dir>/avatars/<owner>.png`. Used by the sidebar to decorate repo
//! workspaces with their GitHub org logo.

use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

pub fn cache_dir() -> PathBuf {
    crate::runtime::data_dir().join("avatars")
}

/// Path where the avatar for `owner` is (or would be) cached. Callers should
/// check `.exists()` before rendering.
pub fn avatar_path(owner: &str) -> PathBuf {
    cache_dir().join(format!("{owner}.png"))
}

/// Download `https://github.com/{owner}.png` to the cache if it isn't already
/// present. No-op if the file exists. Uses blocking reqwest so callers can
/// run it on a plain `std::thread::spawn` without a tokio runtime.
pub fn fetch_blocking(owner: &str) -> Result<PathBuf> {
    let path = avatar_path(owner);
    if path.exists() {
        return Ok(path);
    }
    std::fs::create_dir_all(cache_dir())?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let url = format!("https://{}/{owner}.png", crate::git::GITHUB_HOST);
    let resp = client.get(&url).send()?.error_for_status()?;
    let bytes = resp.bytes()?;
    std::fs::write(&path, &bytes)?;
    Ok(path)
}

fn in_flight() -> &'static Mutex<HashSet<String>> {
    static IN_FLIGHT: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    IN_FLIGHT.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Spawn a background thread to populate the avatar cache for `owner`.
/// Silently swallows errors (the sidebar falls back to not rendering an
/// avatar if the file doesn't exist). De-duplicated: a second call for the
/// same owner while the first is still running is a no-op.
pub fn prefetch(owner: String) {
    {
        let mut set = in_flight().lock().unwrap();
        if !set.insert(owner.clone()) {
            return;
        }
    }
    std::thread::spawn(move || {
        if let Err(e) = fetch_blocking(&owner) {
            eprintln!("avatar prefetch failed for {owner}: {e}");
        }
        in_flight().lock().unwrap().remove(&owner);
    });
}
