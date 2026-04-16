//! Lightweight `.git` directory probing: no git binary shell-outs, no libgit2.
//! Just enough to surface the current branch and the GitHub owner for a repo
//! in the sidebar.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const GITHUB_HOST: &str = "github.com";
pub const GIT_DIR: &str = ".git";

/// Whether `path` is a git repo (either a `.git` directory or a gitdir file
/// pointing at one, for worktrees/submodules).
pub fn is_git_repo(path: &Path) -> bool {
    resolve_git_dir(path).is_some()
}

/// Resolve the real git directory for `repo_path`. Handles both regular repos
/// (`.git/` directory) and worktrees / submodules (`.git` file containing
/// `gitdir: <path>`).
fn resolve_git_dir(repo_path: &Path) -> Option<PathBuf> {
    let dot_git = repo_path.join(GIT_DIR);
    let meta = std::fs::metadata(&dot_git).ok()?;
    if meta.is_dir() {
        return Some(dot_git);
    }
    if meta.is_file() {
        let content = std::fs::read_to_string(&dot_git).ok()?;
        let gitdir = content.trim().strip_prefix("gitdir:")?.trim();
        let path = PathBuf::from(gitdir);
        return Some(if path.is_absolute() {
            path
        } else {
            repo_path.join(path)
        });
    }
    None
}

/// Read the current branch from `HEAD`. Returns the branch name for normal
/// HEADs, a short SHA for detached HEADs, or `None` if the file can't be
/// read.
pub fn read_head_branch(repo_path: &Path) -> Option<String> {
    let git_dir = resolve_git_dir(repo_path)?;
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    if let Some(rest) = head.strip_prefix("ref: refs/heads/") {
        Some(rest.to_string())
    } else if !head.is_empty() {
        Some(head.chars().take(7).collect())
    } else {
        None
    }
}

/// Read the `[remote "origin"]` url from the repo's `config`. Naive but
/// sufficient for the common single-remote case.
pub fn read_origin_url(repo_path: &Path) -> Option<String> {
    let git_dir = resolve_git_dir(repo_path)?;
    let config = std::fs::read_to_string(git_dir.join("config")).ok()?;
    let mut in_origin = false;
    for line in config.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_origin = line == "[remote \"origin\"]";
            continue;
        }
        if in_origin {
            if let Some(rest) = line.strip_prefix("url") {
                let rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '=').trim();
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// Load SSH Host -> HostName aliases from `~/.ssh/config`. Supports the
/// `HostName` / `Hostname` / `hostname` case-insensitive spelling; ignores
/// everything else. Returns an empty slice if the file is missing.
///
/// Cached for the process's lifetime. `~/.ssh/config` rarely changes during
/// a session and reading it per sidebar row adds up.
pub fn ssh_aliases() -> &'static [(String, String)] {
    static CACHE: OnceLock<Vec<(String, String)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let Ok(home) = std::env::var("HOME") else { return Vec::new(); };
        let Ok(config) = std::fs::read_to_string(PathBuf::from(home).join(".ssh").join("config"))
        else { return Vec::new(); };
        parse_ssh_aliases(&config)
    })
}

fn parse_ssh_aliases(config: &str) -> Vec<(String, String)> {
    let mut aliases = Vec::new();
    let mut current_hosts: Vec<String> = Vec::new();
    for raw_line in config.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, |c: char| c.is_whitespace() || c == '=');
        let (key, value) = match (parts.next(), parts.next()) {
            (Some(k), Some(v)) => (k.trim().to_ascii_lowercase(), v.trim().to_string()),
            _ => continue,
        };
        if key == "host" {
            current_hosts = value.split_whitespace().map(|s| s.to_string()).collect();
        } else if key == "hostname" {
            for host in &current_hosts {
                // Skip wildcards — can't be safely expanded.
                if host.contains('*') || host.contains('?') {
                    continue;
                }
                aliases.push((host.clone(), value.clone()));
            }
        }
    }
    aliases
}

/// Extract the GitHub owner from a remote URL, resolving SSH `Host` aliases.
/// Handles SSH, HTTPS, HTTP and `ssh://` forms with optional `.git` suffix,
/// optional port, and case-insensitive hostname.
pub fn parse_github_owner(url: &str, aliases: &[(String, String)]) -> Option<String> {
    let (host, path) = parse_url(url.trim())?;
    let resolved_host = resolve_host(&host, aliases);
    if !resolved_host.eq_ignore_ascii_case(GITHUB_HOST) {
        return None;
    }
    let owner = path.split('/').next()?.trim_end_matches(".git");
    if owner.is_empty() { None } else { Some(owner.to_string()) }
}

fn resolve_host(host: &str, aliases: &[(String, String)]) -> String {
    aliases
        .iter()
        .find(|(alias, _)| alias.eq_ignore_ascii_case(host))
        .map(|(_, real)| real.clone())
        .unwrap_or_else(|| host.to_string())
}

/// Parse a git remote url into `(host, path)`. Path does NOT include the
/// leading separator. Returns `None` if the URL shape isn't recognized.
fn parse_url(url: &str) -> Option<(String, String)> {
    // scp-like form: [user@]host:path
    if !url.contains("://") {
        let (prefix, path) = url.split_once(':')?;
        let host = prefix.rsplit_once('@').map(|(_, h)| h).unwrap_or(prefix);
        return Some((host.to_string(), path.to_string()));
    }
    // url form: scheme://[user@]host[:port]/path
    let (_scheme, rest) = url.split_once("://")?;
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host_port = authority.rsplit_once('@').map(|(_, h)| h).unwrap_or(authority);
    let host = host_port.split(':').next()?;
    Some((host.to_string(), path.to_string()))
}

/// Convenience: read the GitHub owner of the `origin` remote for a repo at
/// `repo_path`, if any. Resolves SSH aliases from `~/.ssh/config`.
pub fn github_owner_for_repo(repo_path: &Path) -> Option<String> {
    parse_github_owner(&read_origin_url(repo_path)?, ssh_aliases())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    /// RAII tempdir: unique-per-test, removed on drop (including panic).
    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "superhq-git-test-{}-{}",
                name,
                std::process::id()
            ));
            if dir.exists() {
                    }
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl std::ops::Deref for ScratchDir {
        type Target = Path;
        fn deref(&self) -> &Path { &self.0 }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).ok();
        }
    }

    fn write_file(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut f = fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    // ── parse_github_owner ──────────────────────────────────────────

    #[test]
    fn owner_from_ssh() {
        assert_eq!(parse_github_owner("git@github.com:superhq-ai/superhq.git", &[]), Some("superhq-ai".into()));
    }

    #[test]
    fn owner_from_ssh_without_git_suffix() {
        assert_eq!(parse_github_owner("git@github.com:torvalds/linux", &[]), Some("torvalds".into()));
    }

    #[test]
    fn owner_from_https() {
        assert_eq!(parse_github_owner("https://github.com/rust-lang/rust.git", &[]), Some("rust-lang".into()));
    }

    #[test]
    fn owner_from_http() {
        assert_eq!(parse_github_owner("http://github.com/foo/bar", &[]), Some("foo".into()));
    }

    #[test]
    fn owner_from_ssh_scheme_with_port() {
        assert_eq!(parse_github_owner("ssh://git@github.com:22/owner/repo.git", &[]), Some("owner".into()));
    }

    #[test]
    fn owner_is_case_insensitive_on_hostname() {
        assert_eq!(parse_github_owner("git@GitHub.com:foo/bar.git", &[]), Some("foo".into()));
        assert_eq!(parse_github_owner("https://GITHUB.COM/foo/bar", &[]), Some("foo".into()));
    }

    #[test]
    fn owner_resolves_ssh_host_alias() {
        let aliases = vec![("github-work".into(), "github.com".into())];
        assert_eq!(parse_github_owner("git@github-work:acme/app.git", &aliases), Some("acme".into()));
    }

    #[test]
    fn owner_rejects_non_github() {
        assert_eq!(parse_github_owner("git@gitlab.com:foo/bar.git", &[]), None);
        assert_eq!(parse_github_owner("https://bitbucket.org/x/y.git", &[]), None);
    }

    #[test]
    fn owner_rejects_empty_owner() {
        assert_eq!(parse_github_owner("https://github.com//foo", &[]), None);
    }

    #[test]
    fn owner_trims_whitespace() {
        assert_eq!(parse_github_owner("  git@github.com:foo/bar.git  ", &[]), Some("foo".into()));
    }

    // ── parse_ssh_aliases ───────────────────────────────────────────

    #[test]
    fn ssh_config_single_host() {
        let cfg = "Host github-work\n  HostName github.com\n  User git\n";
        assert_eq!(parse_ssh_aliases(cfg), vec![("github-work".into(), "github.com".into())]);
    }

    #[test]
    fn ssh_config_multiple_hosts_share_hostname() {
        let cfg = "Host gh gh2\n  HostName github.com\n";
        assert_eq!(
            parse_ssh_aliases(cfg),
            vec![("gh".into(), "github.com".into()), ("gh2".into(), "github.com".into())],
        );
    }

    #[test]
    fn ssh_config_case_insensitive_keys() {
        let cfg = "host wo\n  hostname github.com\n";
        assert_eq!(parse_ssh_aliases(cfg), vec![("wo".into(), "github.com".into())]);
    }

    #[test]
    fn ssh_config_skips_wildcard_hosts() {
        let cfg = "Host *\n  HostName example.com\nHost alias\n  HostName github.com\n";
        assert_eq!(parse_ssh_aliases(cfg), vec![("alias".into(), "github.com".into())]);
    }

    #[test]
    fn ssh_config_ignores_comments_and_blanks() {
        let cfg = "# comment\n\nHost alias\n  HostName github.com\n# end\n";
        assert_eq!(parse_ssh_aliases(cfg), vec![("alias".into(), "github.com".into())]);
    }

    // ── read_head_branch ────────────────────────────────────────────

    #[test]
    fn head_branch_symbolic_ref() {
        let dir = ScratchDir::new("head_symbolic");
        write_file(&dir.join(".git").join("HEAD"), "ref: refs/heads/main\n");
        assert_eq!(read_head_branch(&dir), Some("main".into()));
    }

    #[test]
    fn head_branch_detached_returns_short_sha() {
        let dir = ScratchDir::new("head_detached");
        write_file(&dir.join(".git").join("HEAD"), "1234567890abcdef1234567890abcdef12345678\n");
        assert_eq!(read_head_branch(&dir), Some("1234567".into()));
    }

    #[test]
    fn head_branch_worktree_resolves_gitdir() {
        let dir = ScratchDir::new("head_worktree");
        let real_gitdir = dir.join("real-gitdir");
        write_file(&real_gitdir.join("HEAD"), "ref: refs/heads/feature/x\n");
        write_file(&dir.join(".git"), &format!("gitdir: {}\n", real_gitdir.display()));
        assert_eq!(read_head_branch(&dir), Some("feature/x".into()));
    }

    #[test]
    fn head_branch_missing_returns_none() {
        let dir = ScratchDir::new("head_missing");
        assert_eq!(read_head_branch(&dir), None);
    }

    // ── read_origin_url ─────────────────────────────────────────────

    #[test]
    fn origin_url_simple() {
        let dir = ScratchDir::new("origin_simple");
        write_file(
            &dir.join(".git").join("config"),
            "[remote \"origin\"]\n\turl = git@github.com:foo/bar.git\n",
        );
        assert_eq!(read_origin_url(&dir), Some("git@github.com:foo/bar.git".into()));
    }

    #[test]
    fn origin_url_picks_origin_over_other_remotes() {
        let dir = ScratchDir::new("origin_multiple");
        write_file(
            &dir.join(".git").join("config"),
            "[remote \"upstream\"]\n\turl = https://github.com/upstream/repo.git\n\
             [remote \"origin\"]\n\turl = https://github.com/fork/repo.git\n",
        );
        assert_eq!(read_origin_url(&dir), Some("https://github.com/fork/repo.git".into()));
    }

    #[test]
    fn origin_url_absent_returns_none() {
        let dir = ScratchDir::new("origin_absent");
        write_file(
            &dir.join(".git").join("config"),
            "[core]\n\tbare = false\n[remote \"upstream\"]\n\turl = x\n",
        );
        assert_eq!(read_origin_url(&dir), None);
    }
}
