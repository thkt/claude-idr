use std::process::Command;

/// Returns the staged diff (`git diff --cached`).
/// Returns empty string on failure (fail-open).
pub fn staged_diff() -> String {
    run_git(&["diff", "--cached"])
}

/// Returns the staged diff stat (`git diff --cached --stat`).
/// Returns empty string on failure (fail-open).
pub fn staged_stat() -> String {
    run_git(&["diff", "--cached", "--stat"])
}

fn run_git(args: &[&str]) -> String {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).into_owned())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staged_diff_returns_empty_when_no_staged_changes() {
        // In a clean repo with no staged changes, staged_diff should return ""
        let result = staged_diff();
        assert!(
            result.is_empty(),
            "Expected empty string when no staged changes, got: {:?}",
            result
        );
    }

    #[test]
    fn staged_stat_returns_empty_when_no_staged_changes() {
        // In a clean repo with no staged changes, staged_stat should return ""
        let result = staged_stat();
        assert!(
            result.is_empty(),
            "Expected empty string when no staged changes, got: {:?}",
            result
        );
    }
}
