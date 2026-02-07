use serde_json::Value;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Extract context from a session JSONL file.
///
/// Returns a formatted string containing:
/// - Changed files (from Write/Edit tool uses)
/// - User requests (first 150 chars of each user text message)
///
/// Returns None if the file cannot be read or contains no relevant data.
pub fn extract(session: &Path) -> Option<String> {
    let file = File::open(session).ok()?;
    let reader = BufReader::new(file);

    let mut changed_files: BTreeSet<String> = BTreeSet::new();
    let mut user_requests: Vec<String> = Vec::new();

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

        // Extract changed files from Write/Edit tool uses
        extract_changed_files(&v, &mut changed_files);

        // Extract user text messages
        extract_user_request(&v, &mut user_requests);
    }

    if changed_files.is_empty() && user_requests.is_empty() {
        return None;
    }

    let mut output = String::new();

    output.push_str("# Changed files:\n");
    for file_path in &changed_files {
        output.push_str(&format!("- {file_path}\n"));
    }

    output.push('\n');
    output.push_str("# User requests in this session:\n");
    for req in user_requests.iter().take(20) {
        output.push_str(&format!("- {req}\n"));
    }

    Some(output)
}

/// Extract file paths from Write/Edit tool uses in a JSONL line.
fn extract_changed_files(v: &Value, out: &mut BTreeSet<String>) {
    let content = match v.pointer("/message/content") {
        Some(c) => c,
        None => return,
    };
    let arr = match content.as_array() {
        Some(a) => a,
        None => return,
    };
    for item in arr {
        let name = match item.get("name").and_then(|n| n.as_str()) {
            Some(n) => n,
            None => continue,
        };
        if (name == "Write" || name == "Edit")
            && let Some(file_path) = item.pointer("/input/file_path").and_then(|p| p.as_str())
        {
            out.insert(file_path.to_string());
        }
    }
}

/// Extract user text messages (where type == "user" and content is a string).
fn extract_user_request(v: &Value, out: &mut Vec<String>) {
    if v.get("type").and_then(|t| t.as_str()) != Some("user") {
        return;
    }
    if let Some(content) = v.pointer("/message/content").and_then(|c| c.as_str()) {
        let truncated: String = content.chars().take(150).collect();
        out.push(truncated);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_jsonl(dir: &Path, name: &str, lines: &[&str]) -> std::path::PathBuf {
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

    // -- extract tests --

    #[test]
    fn extract_returns_none_for_nonexistent_file() {
        assert!(extract(Path::new("/nonexistent/session.jsonl")).is_none());
    }

    #[test]
    fn extract_returns_none_for_empty_file() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(dir.path(), "empty.jsonl", &[]);
        assert!(extract(&jsonl).is_none());
    }

    #[test]
    fn extract_returns_none_when_no_relevant_data() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "irrelevant.jsonl",
            &[r#"{"message":{"content":[{"name":"Read","input":{}}]}}"#],
        );
        assert!(extract(&jsonl).is_none());
    }

    #[test]
    fn extract_collects_changed_files() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "session.jsonl",
            &[
                r#"{"message":{"content":[{"name":"Write","input":{"file_path":"src/main.rs"}}]}}"#,
                r#"{"message":{"content":[{"name":"Edit","input":{"file_path":"src/lib.rs"}}]}}"#,
            ],
        );

        let result = extract(&jsonl).unwrap();
        assert!(result.contains("# Changed files:"));
        assert!(result.contains("- src/lib.rs"));
        assert!(result.contains("- src/main.rs"));
    }

    #[test]
    fn extract_deduplicates_changed_files() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "session.jsonl",
            &[
                r#"{"message":{"content":[{"name":"Write","input":{"file_path":"src/main.rs"}}]}}"#,
                r#"{"message":{"content":[{"name":"Edit","input":{"file_path":"src/main.rs"}}]}}"#,
            ],
        );

        let result = extract(&jsonl).unwrap();
        // Count occurrences of "- src/main.rs"
        let count = result.matches("- src/main.rs").count();
        assert_eq!(count, 1, "Duplicate file paths should be deduplicated");
    }

    #[test]
    fn extract_collects_user_requests() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "session.jsonl",
            &[
                r#"{"type":"user","message":{"content":"fix the bug in auth module"}}"#,
                r#"{"message":{"content":[{"name":"Write","input":{"file_path":"src/auth.rs"}}]}}"#,
                r#"{"type":"user","message":{"content":"looks good, thanks"}}"#,
            ],
        );

        let result = extract(&jsonl).unwrap();
        assert!(result.contains("# User requests in this session:"));
        assert!(result.contains("- fix the bug in auth module"));
        assert!(result.contains("- looks good, thanks"));
    }

    #[test]
    fn extract_truncates_long_user_messages() {
        let dir = TempDir::new().unwrap();
        let long_msg = "a".repeat(300);
        let line = format!(r#"{{"type":"user","message":{{"content":"{long_msg}"}}}}"#);
        // Need at least one Write/Edit to not return None (user request alone is enough)
        let jsonl = write_jsonl(dir.path(), "session.jsonl", &[&line]);

        let result = extract(&jsonl).unwrap();
        // The truncated message should be 150 chars
        let expected_truncated = "a".repeat(150);
        assert!(result.contains(&expected_truncated));
        // Should NOT contain 151+ 'a's in a single line
        let too_long = "a".repeat(151);
        assert!(!result.contains(&too_long));
    }

    #[test]
    fn extract_skips_non_string_user_content() {
        // When user message content is an array (not string), skip it
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "session.jsonl",
            &[
                r#"{"type":"user","message":{"content":[{"type":"image","data":"..."}]}}"#,
                r#"{"message":{"content":[{"name":"Write","input":{"file_path":"x.rs"}}]}}"#,
            ],
        );

        let result = extract(&jsonl).unwrap();
        assert!(result.contains("# Changed files:"));
        assert!(result.contains("- x.rs"));
        // No user requests since content was an array
        assert!(!result.contains("image"));
    }

    #[test]
    fn extract_handles_mixed_valid_and_invalid_json() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "session.jsonl",
            &[
                "not valid json at all",
                r#"{"message":{"content":[{"name":"Edit","input":{"file_path":"a.rs"}}]}}"#,
                "{broken",
                r#"{"type":"user","message":{"content":"hello"}}"#,
            ],
        );

        let result = extract(&jsonl).unwrap();
        assert!(result.contains("- a.rs"));
        assert!(result.contains("- hello"));
    }

    #[test]
    fn extract_output_format_matches_spec() {
        let dir = TempDir::new().unwrap();
        let jsonl = write_jsonl(
            dir.path(),
            "session.jsonl",
            &[
                r#"{"type":"user","message":{"content":"add feature X"}}"#,
                r#"{"message":{"content":[{"name":"Write","input":{"file_path":"src/foo.ts"}}]}}"#,
                r#"{"message":{"content":[{"name":"Edit","input":{"file_path":"src/bar.ts"}}]}}"#,
            ],
        );

        let result = extract(&jsonl).unwrap();
        let expected = "\
# Changed files:
- src/bar.ts
- src/foo.ts

# User requests in this session:
- add feature X
";
        assert_eq!(result, expected);
    }

    // -- extract_changed_files tests --

    #[test]
    fn extract_changed_files_ignores_non_write_edit_tools() {
        let v: Value = serde_json::from_str(
            r#"{"message":{"content":[{"name":"Bash","input":{"command":"ls"}}]}}"#,
        )
        .unwrap();
        let mut files = BTreeSet::new();
        extract_changed_files(&v, &mut files);
        assert!(files.is_empty());
    }

    #[test]
    fn extract_changed_files_handles_missing_file_path() {
        let v: Value =
            serde_json::from_str(r#"{"message":{"content":[{"name":"Write","input":{}}]}}"#)
                .unwrap();
        let mut files = BTreeSet::new();
        extract_changed_files(&v, &mut files);
        assert!(files.is_empty());
    }

    // -- extract_user_request tests --

    #[test]
    fn extract_user_request_ignores_non_user_type() {
        let v: Value =
            serde_json::from_str(r#"{"type":"assistant","message":{"content":"sure, I'll help"}}"#)
                .unwrap();
        let mut requests = Vec::new();
        extract_user_request(&v, &mut requests);
        assert!(requests.is_empty());
    }

    #[test]
    fn extract_user_request_ignores_missing_type() {
        let v: Value = serde_json::from_str(r#"{"message":{"content":"orphan message"}}"#).unwrap();
        let mut requests = Vec::new();
        extract_user_request(&v, &mut requests);
        assert!(requests.is_empty());
    }
}
