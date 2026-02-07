use crate::jsonl;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub fn extract(session: &Path) -> Option<String> {
    let mut changed_files = BTreeSet::new();
    let mut user_requests = Vec::new();

    for v in jsonl::iter_values(session) {
        extract_changed_files(&v, &mut changed_files);
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
    const MAX_USER_REQUESTS: usize = 20;
    for req in user_requests.iter().take(MAX_USER_REQUESTS) {
        output.push_str(&format!("- {req}\n"));
    }

    Some(output)
}

fn extract_changed_files(v: &Value, out: &mut BTreeSet<String>) {
    let Some(arr) = v.pointer("/message/content").and_then(|c| c.as_array()) else {
        return;
    };
    for item in arr {
        if matches!(
            item.get("name").and_then(|n| n.as_str()),
            Some("Write" | "Edit")
        ) && let Some(file_path) = item.pointer("/input/file_path").and_then(|p| p.as_str())
        {
            out.insert(file_path.to_string());
        }
    }
}

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
    use crate::testutil::write_jsonl;
    use tempfile::TempDir;

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
        let count = result.matches("- src/main.rs").count();
        assert_eq!(count, 1);
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
        let jsonl = write_jsonl(dir.path(), "session.jsonl", &[&line]);

        let result = extract(&jsonl).unwrap();
        let expected_truncated = "a".repeat(150);
        assert!(result.contains(&expected_truncated));
        let too_long = "a".repeat(151);
        assert!(!result.contains(&too_long));
    }

    #[test]
    fn extract_skips_non_string_user_content() {
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

    #[test]
    fn extract_truncates_user_requests_at_max() {
        let dir = TempDir::new().unwrap();
        let lines: Vec<String> = (0..25)
            .map(|i| format!(r#"{{"type":"user","message":{{"content":"request {i}"}}}}"#))
            .collect();
        let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        let jsonl = write_jsonl(dir.path(), "session.jsonl", &line_refs);

        let result = extract(&jsonl).unwrap();
        let count = result.matches("\n- request ").count();
        assert_eq!(count, 20);
        assert!(result.contains("- request 0"));
        assert!(result.contains("- request 19"));
        assert!(!result.contains("- request 20"));
    }
}
