use crate::config::Config;
use std::fs;
use std::path::{Path, PathBuf};

/// Resolve the IDR output directory based on config.
///
/// Resolution order:
/// 1. Check `workspace_dir/.current-sow` for a valid SOW file path
/// 2. If valid (exists, within workspace_dir), use that SOW file's parent directory
/// 3. Otherwise, fall back to `workspace_dir/planning/YYYY-MM-DD/`
///
/// Creates the directory if it doesn't exist. Fail-open: defaults to date-based path on error.
pub fn resolve(config: &Config) -> PathBuf {
    resolve_with_date(config, &today_date())
}

/// Testable variant that accepts an explicit date string.
fn resolve_with_date(config: &Config, date: &str) -> PathBuf {
    let sow_file = config.workspace_dir.join(".current-sow");

    if let Ok(sow_content) = fs::read_to_string(&sow_file) {
        let sow_path = PathBuf::from(sow_content.trim());
        if let Some(dir) = validate_sow_path(&sow_path, &config.workspace_dir) {
            let _ = fs::create_dir_all(&dir);
            return dir;
        }
    }

    let date_dir = config.workspace_dir.join("planning").join(date);
    let _ = fs::create_dir_all(&date_dir);
    date_dir
}

/// Validate that the SOW path points to an existing file within the workspace directory.
/// Returns the SOW file's parent directory if valid, None otherwise.
fn validate_sow_path(sow_path: &Path, workspace_dir: &Path) -> Option<PathBuf> {
    let real_sow = fs::canonicalize(sow_path).ok()?;
    let real_workspace = fs::canonicalize(workspace_dir).ok()?;

    if !real_sow.starts_with(&real_workspace) {
        return None;
    }
    if !real_sow.is_file() {
        return None;
    }

    real_sow.parent().map(PathBuf::from)
}

/// Find the next IDR number by scanning the directory for `idr-*.md` files.
/// Returns max_existing + 1, or 1 if no IDR files exist.
pub fn next_number(dir: &Path) -> u32 {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 1,
    };

    let max = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name();
            let name = name.to_str()?;
            parse_idr_number(name)
        })
        .max()
        .unwrap_or(0);

    max + 1
}

/// Parse the numeric part from an IDR filename like "idr-01.md" -> Some(1).
fn parse_idr_number(filename: &str) -> Option<u32> {
    let stem = filename.strip_prefix("idr-")?.strip_suffix(".md")?;
    stem.parse::<u32>().ok()
}

/// Write an IDR file with the standard format.
///
/// Format:
/// ```text
/// # IDR: {purpose}
///
/// > {date}
///
/// {content}
///
/// ---
///
/// ### git diff --stat
/// ```
/// {stat}
/// ```
/// ```
pub fn write_idr(path: &Path, purpose: &Option<String>, content: &str, stat: &str) {
    let purpose_text = purpose.as_deref().unwrap_or("(目的抽出失敗)");
    let datetime = now_datetime();

    let body = format!(
        "# IDR: {purpose_text}\n\n\
         > {datetime}\n\n\
         {content}\n\n\
         ---\n\n\
         ### git diff --stat\n\
         ```\n{stat}\n```\n"
    );

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(e) = fs::write(path, &body) {
        eprintln!(
            "claude-idr: warning: failed to write IDR {}: {}",
            path.display(),
            e
        );
    }
}

/// Returns today's date as YYYY-MM-DD.
fn today_date() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_date_from_epoch(secs)
}

/// Returns current datetime as "YYYY-MM-DD HH:MM".
fn now_datetime() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_datetime_from_epoch(secs)
}

/// Convert epoch seconds to YYYY-MM-DD using civil date calculation.
fn format_date_from_epoch(epoch_secs: u64) -> String {
    let (y, m, d) = epoch_to_civil(epoch_secs);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Convert epoch seconds to "YYYY-MM-DD HH:MM" using civil date calculation.
fn format_datetime_from_epoch(epoch_secs: u64) -> String {
    let (y, m, d) = epoch_to_civil(epoch_secs);
    let day_secs = epoch_secs % 86400;
    let h = day_secs / 3600;
    let min = (day_secs % 3600) / 60;
    format!("{y:04}-{m:02}-{d:02} {h:02}:{min:02}")
}

/// Convert Unix epoch seconds to (year, month, day) in UTC.
/// Algorithm from Howard Hinnant's civil_from_days.
fn epoch_to_civil(epoch_secs: u64) -> (i32, u32, u32) {
    let z = (epoch_secs / 86400) as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // --- resolve tests ---

    #[test]
    fn resolve_uses_date_based_path_when_no_current_sow() {
        let tmp = TempDir::new().unwrap();
        let config = Config {
            workspace_dir: tmp.path().to_path_buf(),
            ..Config::default()
        };

        let result = resolve_with_date(&config, "2026-02-07");

        let expected = tmp.path().join("planning").join("2026-02-07");
        assert_eq!(result, expected);
        assert!(result.is_dir(), "directory should be created");
    }

    #[test]
    fn resolve_uses_sow_directory_when_valid_current_sow() {
        let tmp = TempDir::new().unwrap();
        let sow_dir = tmp.path().join("sow").join("project-x");
        fs::create_dir_all(&sow_dir).unwrap();

        // Create the SOW file
        let sow_file = sow_dir.join("sow.md");
        fs::write(&sow_file, "# SOW").unwrap();

        // Write .current-sow pointing to it
        let current_sow = tmp.path().join(".current-sow");
        fs::write(&current_sow, sow_file.to_str().unwrap()).unwrap();

        let config = Config {
            workspace_dir: tmp.path().to_path_buf(),
            ..Config::default()
        };

        let result = resolve_with_date(&config, "2026-02-07");

        // Should resolve to the SOW file's parent directory
        assert_eq!(result, fs::canonicalize(&sow_dir).unwrap());
    }

    #[test]
    fn resolve_falls_back_to_date_when_sow_points_outside_workspace() {
        let workspace = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();

        // Create a file outside workspace
        let outside_file = outside.path().join("sow.md");
        fs::write(&outside_file, "# SOW").unwrap();

        // Write .current-sow pointing outside
        let current_sow = workspace.path().join(".current-sow");
        fs::write(&current_sow, outside_file.to_str().unwrap()).unwrap();

        let config = Config {
            workspace_dir: workspace.path().to_path_buf(),
            ..Config::default()
        };

        let result = resolve_with_date(&config, "2026-02-07");

        let expected = workspace.path().join("planning").join("2026-02-07");
        assert_eq!(result, expected);
    }

    #[test]
    fn resolve_falls_back_to_date_when_sow_file_does_not_exist() {
        let tmp = TempDir::new().unwrap();

        // Write .current-sow pointing to a nonexistent file
        let current_sow = tmp.path().join(".current-sow");
        fs::write(&current_sow, "/nonexistent/path/sow.md").unwrap();

        let config = Config {
            workspace_dir: tmp.path().to_path_buf(),
            ..Config::default()
        };

        let result = resolve_with_date(&config, "2026-02-07");

        let expected = tmp.path().join("planning").join("2026-02-07");
        assert_eq!(result, expected);
    }

    // --- next_number tests ---

    #[test]
    fn next_number_returns_1_for_empty_directory() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(next_number(tmp.path()), 1);
    }

    #[test]
    fn next_number_returns_max_plus_1() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("idr-01.md"), "content").unwrap();
        fs::write(tmp.path().join("idr-03.md"), "content").unwrap();

        assert_eq!(next_number(tmp.path()), 4);
    }

    #[test]
    fn next_number_ignores_non_idr_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("idr-02.md"), "content").unwrap();
        fs::write(tmp.path().join("notes.md"), "other").unwrap();
        fs::write(tmp.path().join("readme.txt"), "other").unwrap();
        fs::write(tmp.path().join("idr-summary.md"), "other").unwrap();

        assert_eq!(next_number(tmp.path()), 3);
    }

    #[test]
    fn next_number_returns_1_for_nonexistent_directory() {
        let tmp = TempDir::new().unwrap();
        let nonexistent = tmp.path().join("does-not-exist");
        assert_eq!(next_number(&nonexistent), 1);
    }

    #[test]
    fn next_number_handles_large_numbers() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("idr-99.md"), "content").unwrap();

        assert_eq!(next_number(tmp.path()), 100);
    }

    // --- parse_idr_number tests ---

    #[test]
    fn parse_idr_number_extracts_number() {
        assert_eq!(parse_idr_number("idr-01.md"), Some(1));
        assert_eq!(parse_idr_number("idr-42.md"), Some(42));
        assert_eq!(parse_idr_number("idr-100.md"), Some(100));
    }

    #[test]
    fn parse_idr_number_rejects_invalid_names() {
        assert_eq!(parse_idr_number("notes.md"), None);
        assert_eq!(parse_idr_number("idr-.md"), None);
        assert_eq!(parse_idr_number("idr-abc.md"), None);
        assert_eq!(parse_idr_number("idr-01.txt"), None);
    }

    // --- write_idr tests ---

    #[test]
    fn write_idr_creates_file_with_correct_format() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("idr-01.md");
        let purpose = Some("テスト目的".to_string());
        let content = "## 変更概要\n\nテスト内容";
        let stat = " src/main.rs | 10 +++++++---";

        write_idr(&path, &purpose, content, stat);

        let result = fs::read_to_string(&path).unwrap();
        assert!(result.starts_with("# IDR: テスト目的\n\n> "));
        assert!(result.contains(content));
        assert!(result.contains("---\n\n### git diff --stat\n```\n"));
        assert!(result.contains(stat));
        assert!(result.ends_with("```\n"));
    }

    #[test]
    fn write_idr_uses_fallback_purpose_when_none() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("idr-01.md");

        write_idr(&path, &None, "content", "stat");

        let result = fs::read_to_string(&path).unwrap();
        assert!(result.starts_with("# IDR: (目的抽出失敗)"));
    }

    #[test]
    fn write_idr_creates_parent_directories() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nested").join("dir").join("idr-01.md");

        write_idr(&path, &None, "content", "stat");

        assert!(path.exists(), "file should be created with parent dirs");
    }

    // --- date formatting tests ---

    #[test]
    fn format_date_from_epoch_returns_correct_date() {
        // 2026-02-07 00:00:00 UTC = 1770422400
        assert_eq!(format_date_from_epoch(1770422400), "2026-02-07");
    }

    #[test]
    fn format_datetime_from_epoch_returns_correct_datetime() {
        // 2026-02-07 14:30:00 UTC = 1770422400 + 14*3600 + 30*60
        let epoch = 1770422400 + 14 * 3600 + 30 * 60;
        assert_eq!(format_datetime_from_epoch(epoch), "2026-02-07 14:30");
    }

    #[test]
    fn epoch_to_civil_handles_epoch_zero() {
        assert_eq!(epoch_to_civil(0), (1970, 1, 1));
    }

    #[test]
    fn epoch_to_civil_handles_known_date() {
        // 2000-01-01 00:00:00 UTC = 946684800
        assert_eq!(epoch_to_civil(946684800), (2000, 1, 1));
    }
}
