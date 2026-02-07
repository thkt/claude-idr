use crate::config::Config;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Find the most recently modified .jsonl file under `~/.claude/projects/`
/// that was modified within `config.session_max_age_min` minutes.
/// Excludes files in `subagents/` subdirectories.
pub fn find_recent(config: &Config) -> Option<PathBuf> {
    let project_dir = dirs::home_dir()?.join(".claude").join("projects");
    if !project_dir.is_dir() {
        return None;
    }

    let max_age = std::time::Duration::from_secs(config.session_max_age_min * 60);
    let now = SystemTime::now();

    let mut candidates: Vec<(PathBuf, SystemTime)> = Vec::new();
    collect_jsonl_files(&project_dir, &mut candidates);

    // Filter by age and subagents exclusion, then pick the most recent
    candidates
        .into_iter()
        .filter(|(path, mtime)| {
            // Exclude subagents/ paths
            !path_contains_subagents(path)
                && now.duration_since(*mtime).is_ok_and(|age| age <= max_age)
        })
        .max_by_key(|(_, mtime)| *mtime)
        .map(|(path, _)| path)
}

/// Check if any line in the JSONL file contains a Write or Edit tool use.
/// Returns false on any error (fail-open).
pub fn has_write_or_edit(path: &Path) -> bool {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if has_tool_name(&v, &["Write", "Edit"]) {
            return true;
        }
    }
    false
}

/// Check if `message.content[].name` matches any of the given tool names.
fn has_tool_name(v: &Value, tool_names: &[&str]) -> bool {
    if let Some(content) = v.pointer("/message/content")
        && let Some(arr) = content.as_array()
    {
        for item in arr {
            if let Some(name) = item.get("name").and_then(|n| n.as_str())
                && tool_names.contains(&name)
            {
                return true;
            }
        }
    }
    false
}

/// Recursively collect .jsonl files with their modification times.
fn collect_jsonl_files(dir: &Path, out: &mut Vec<(PathBuf, SystemTime)>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
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

/// Check if a path contains a "subagents" component.
fn path_contains_subagents(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == "subagents")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_jsonl(dir: &Path, name: &str, lines: &[&str]) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut file = File::create(&path).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        path
    }

    // -- has_write_or_edit tests --

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
        // User messages have content as a string, not array
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

    // -- has_tool_name tests --

    #[test]
    fn has_tool_name_returns_false_for_no_message() {
        let v: Value = serde_json::from_str(r#"{"type":"system"}"#).unwrap();
        assert!(!has_tool_name(&v, &["Write"]));
    }

    #[test]
    fn has_tool_name_returns_false_for_string_content() {
        let v: Value = serde_json::from_str(r#"{"message":{"content":"hello"}}"#).unwrap();
        assert!(!has_tool_name(&v, &["Write"]));
    }

    // -- path_contains_subagents tests --

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

    // -- find_recent tests --

    #[test]
    fn find_recent_returns_none_when_no_home_dir_projects() {
        // With default config, find_recent looks at ~/.claude/projects/
        // which may or may not exist. We just verify it doesn't panic.
        let config = Config::default();
        let _ = find_recent(&config);
    }
}
