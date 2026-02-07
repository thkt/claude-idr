use std::process::Command;

pub fn staged_diff() -> Option<String> {
    run_git(&["diff", "--cached"])
}

pub fn staged_stat() -> String {
    run_git(&["diff", "--cached", "--stat"]).unwrap_or_default()
}

pub fn staged_changed_lines() -> u64 {
    run_git(&["diff", "--cached", "-M", "--numstat"])
        .map(|s| parse_numstat(&s))
        .unwrap_or(0)
}

fn parse_numstat(output: &str) -> u64 {
    output
        .lines()
        .map(|line| {
            let mut parts = line.split('\t');
            let added: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let deleted: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            added + deleted
        })
        .sum()
}

fn run_git(args: &[&str]) -> Option<String> {
    match Command::new("git").args(args).output() {
        Ok(o) if o.status.success() => Some(String::from_utf8_lossy(&o.stdout).into_owned()),
        Ok(o) => {
            eprintln!(
                "claude-idr: git error: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            None
        }
        Err(e) => {
            eprintln!("claude-idr: cannot run git: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_numstat_empty_input() {
        assert_eq!(parse_numstat(""), 0);
    }

    #[test]
    fn parse_numstat_single_file() {
        assert_eq!(parse_numstat("10\t5\tsrc/main.rs"), 15);
    }

    #[test]
    fn parse_numstat_multiple_files() {
        let input = "10\t5\tsrc/main.rs\n3\t1\tsrc/lib.rs\n";
        assert_eq!(parse_numstat(input), 19);
    }

    #[test]
    fn parse_numstat_binary_files() {
        // git outputs "-\t-\timage.png" for binary files
        assert_eq!(parse_numstat("-\t-\timage.png"), 0);
    }

    #[test]
    fn parse_numstat_mixed_binary_and_text() {
        let input = "10\t2\tsrc/main.rs\n-\t-\timage.png\n5\t0\tREADME.md";
        assert_eq!(parse_numstat(input), 17);
    }
}
