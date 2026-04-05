use crate::db::{DiffFileStat, DiffStats, GitStatus};

/// Parse `git status --porcelain` output into a GitStatus.
pub fn parse_git_status(output: &str) -> GitStatus {
    let mut status = GitStatus::default();
    for line in output.lines() {
        if line.len() < 2 {
            continue;
        }
        let index = line.as_bytes()[0];
        let worktree = line.as_bytes()[1];

        match (index, worktree) {
            (b'?', b'?') => status.untracked += 1,
            (b' ', _) if worktree != b' ' => status.unstaged += 1,
            (_, b' ') if index != b' ' => status.staged += 1,
            (_, _) if index != b' ' && worktree != b' ' => {
                status.staged += 1;
                status.unstaged += 1;
            }
            _ => {}
        }
    }
    status
}

/// Parse `git diff --stat HEAD` output into DiffStats.
pub fn parse_diff_stats(output: &str) -> DiffStats {
    let mut stats = DiffStats::default();
    for line in output.lines() {
        // Lines look like: " src/main.rs | 10 ++++---"
        if let Some(pipe_pos) = line.find('|') {
            let path = line[..pipe_pos].trim().to_string();
            let rest = &line[pipe_pos + 1..];
            let additions = rest.matches('+').count() as u32;
            let deletions = rest.matches('-').count() as u32;
            stats.additions += additions;
            stats.deletions += deletions;
            stats.files.push(DiffFileStat {
                path,
                additions,
                deletions,
            });
        }
    }
    stats
}

/// Parse `git rev-list --count origin/{base}..HEAD` for ahead count.
pub fn parse_ahead_count(output: &str) -> u32 {
    output.trim().parse().unwrap_or(0)
}

/// Parse `git rev-list --count HEAD..origin/{base}` for behind count.
pub fn parse_behind_count(output: &str) -> u32 {
    output.trim().parse().unwrap_or(0)
}

/// Generate the full git status polling commands to run inside a sandbox.
pub fn git_status_commands(base_branch: &str) -> Vec<&'static str> {
    // These will be run via sandbox.exec_shell()
    // Caller should substitute the base branch
    let _ = base_branch;
    vec![
        "git status --porcelain",
        "git diff --stat HEAD",
        // ahead/behind handled separately with dynamic branch name
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_status() {
        let output = "?? new_file.rs\n M modified.rs\nA  staged.rs\nMM both.rs\n";
        let status = parse_git_status(output);
        assert_eq!(status.untracked, 1);
        assert_eq!(status.unstaged, 2); // ' M' modified.rs + 'MM' both.rs worktree change
        assert_eq!(status.staged, 2);   // 'A ' staged.rs + 'MM' both.rs index change
    }

    #[test]
    fn test_parse_diff_stats() {
        let output = " src/main.rs | 5 +++--\n src/lib.rs  | 3 ++-\n";
        let stats = parse_diff_stats(output);
        assert_eq!(stats.additions, 5); // 3 + 2
        assert_eq!(stats.deletions, 3); // 2 + 1
        assert_eq!(stats.files.len(), 2);
    }
}
