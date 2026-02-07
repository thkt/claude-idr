use crate::config::Config;
use crate::jsonl;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn find_recent(config: &Config) -> Option<PathBuf> {
    let project_dir = dirs::home_dir()?.join(".claude").join("projects");
    find_recent_in(config, SystemTime::now(), &project_dir)
}

fn find_recent_in(config: &Config, now: SystemTime, project_dir: &Path) -> Option<PathBuf> {
    if !project_dir.is_dir() {
        return None;
    }

    let max_age = std::time::Duration::from_secs(config.session_max_age_min * 60);

    let mut candidates = Vec::new();
    collect_jsonl_files(project_dir, &mut candidates);

    candidates
        .into_iter()
        .filter(|(path, mtime)| {
            !path_contains_subagents(path)
                && now.duration_since(*mtime).is_ok_and(|age| age <= max_age)
        })
        .max_by_key(|(_, mtime)| *mtime)
        .map(|(path, _)| path)
}

pub fn has_write_or_edit(path: &Path) -> bool {
    jsonl::iter_values(path).any(|v| {
        v.pointer("/message/content")
            .and_then(|c| c.as_array())
            .is_some_and(|arr| {
                arr.iter().any(|item| {
                    matches!(
                        item.get("name").and_then(|n| n.as_str()),
                        Some("Write" | "Edit")
                    )
                })
            })
    })
}

fn collect_jsonl_files(dir: &Path, out: &mut Vec<(PathBuf, SystemTime)>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "claude-idr: warning: cannot read directory {}: {e}",
                dir.display()
            );
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl")
            && let Ok(meta) = path.metadata()
            && let Ok(mtime) = meta.modified()
        {
            out.push((path, mtime));
        }
    }
}

fn path_contains_subagents(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == "subagents")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::write_jsonl;
    use tempfile::TempDir;

    #[test]
    fn has_write_or_edit_returns_true_for_write_tool() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "session.jsonl",
            &[r#"{"message":{"content":[{"name":"Write","input":{"file_path":"src/main.rs"}}]}}"#],
        );
        assert!(has_write_or_edit(&jsonl));
    }

    #[test]
    fn has_write_or_edit_returns_true_for_edit_tool() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "session.jsonl",
            &[r#"{"message":{"content":[{"name":"Edit","input":{"file_path":"src/lib.rs"}}]}}"#],
        );
        assert!(has_write_or_edit(&jsonl));
    }

    #[test]
    fn has_write_or_edit_returns_false_for_other_tools() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "session.jsonl",
            &[r#"{"message":{"content":[{"name":"Read","input":{"file_path":"src/main.rs"}}]}}"#],
        );
        assert!(!has_write_or_edit(&jsonl));
    }

    #[test]
    fn has_write_or_edit_returns_false_for_empty_file() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(dir.path(), "empty.jsonl", &[]);
        assert!(!has_write_or_edit(&jsonl));
    }

    #[test]
    fn has_write_or_edit_returns_false_for_nonexistent_file() {
        assert!(!has_write_or_edit(Path::new("/nonexistent/path.jsonl")));
    }

    #[test]
    fn has_write_or_edit_skips_invalid_json_lines() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "mixed.jsonl",
            &[
                "not valid json",
                r#"{"message":{"content":[{"name":"Write","input":{"file_path":"x.rs"}}]}}"#,
            ],
        );
        assert!(has_write_or_edit(&jsonl));
    }

    #[test]
    fn has_write_or_edit_handles_user_text_message() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "user.jsonl",
            &[r#"{"type":"user","message":{"content":"fix the bug"}}"#],
        );
        assert!(!has_write_or_edit(&jsonl));
    }

    #[test]
    fn has_write_or_edit_finds_tool_among_multiple_lines() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "multi.jsonl",
            &[
                r#"{"type":"user","message":{"content":"do something"}}"#,
                r#"{"message":{"content":[{"name":"Read","input":{}}]}}"#,
                r#"{"message":{"content":[{"name":"Bash","input":{}}]}}"#,
                r#"{"message":{"content":[{"name":"Edit","input":{"file_path":"a.rs"}}]}}"#,
                r#"{"type":"user","message":{"content":"thanks"}}"#,
            ],
        );
        assert!(has_write_or_edit(&jsonl));
    }

    #[test]
    fn path_contains_subagents_detects_subagents() {
        let path = Path::new("/home/user/.claude/projects/foo/subagents/session.jsonl");
        assert!(path_contains_subagents(path));
    }

    #[test]
    fn path_contains_subagents_passes_normal_path() {
        let path = Path::new("/home/user/.claude/projects/foo/session.jsonl");
        assert!(!path_contains_subagents(path));
    }

    #[test]
    fn find_recent_in_returns_none_for_empty_dir() {
        let dir = TempDir::new().unwrap();
        let config = Config::default();
        let now = SystemTime::now();
        assert!(find_recent_in(&config, now, dir.path()).is_none());
    }

    #[test]
    fn find_recent_in_finds_most_recent_jsonl() {
        let dir = TempDir::new().unwrap();
        write_jsonl(dir.path(), "old.jsonl", &[r#"{"a":1}"#]);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let newer = write_jsonl(dir.path(), "new.jsonl", &[r#"{"b":2}"#]);

        let config = Config::default();
        let result = find_recent_in(&config, SystemTime::now(), dir.path());
        assert_eq!(result, Some(newer));
    }

    #[test]
    fn find_recent_in_excludes_subagents() {
        let dir = TempDir::new().unwrap();
        write_jsonl(dir.path(), "subagents/agent.jsonl", &[r#"{"a":1}"#]);
        let main = write_jsonl(dir.path(), "main.jsonl", &[r#"{"b":2}"#]);

        let config = Config::default();
        let result = find_recent_in(&config, SystemTime::now(), dir.path());
        assert_eq!(result, Some(main));
    }

    #[test]
    fn find_recent_in_respects_max_age() {
        let dir = TempDir::new().unwrap();
        write_jsonl(dir.path(), "session.jsonl", &[r#"{"a":1}"#]);

        let mut config = Config::default();
        config.session_max_age_min = 0; // 0 min = everything is too old
        let future = SystemTime::now() + std::time::Duration::from_secs(120);
        assert!(find_recent_in(&config, future, dir.path()).is_none());
    }
}
