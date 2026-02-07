use crate::config::Config;
use std::fs;
use std::path::{Path, PathBuf};

pub fn resolve(config: &Config) -> PathBuf {
    resolve_with_date(config, &today_date())
}

fn resolve_with_date(config: &Config, date: &str) -> PathBuf {
    if let Some(ref dir) = config.output_dir {
        create_dir_warn(dir);
        return dir.clone();
    }

    let sow_file = config.workspace_dir.join(".current-sow");

    if let Ok(sow_content) = fs::read_to_string(&sow_file) {
        let sow_path = PathBuf::from(sow_content.trim());
        if let Some(dir) = validate_sow_path(&sow_path, &config.workspace_dir) {
            create_dir_warn(&dir);
            return dir;
        }
    }

    let date_dir = config.workspace_dir.join("planning").join(date);
    create_dir_warn(&date_dir);
    date_dir
}

fn create_dir_warn(dir: &Path) {
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!(
            "claude-idr: warning: cannot create directory {}: {e}",
            dir.display()
        );
    }
}

// SAFETY: canonicalize + is_file has a TOCTOU gap, but the worst case is
// writing the IDR to a stale directory, which is harmless for this use case.
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

fn parse_idr_number(filename: &str) -> Option<u32> {
    let stem = filename.strip_prefix("idr-")?.strip_suffix(".md")?;
    stem.parse::<u32>().ok()
}

pub fn write_idr(path: &Path, purpose: &Option<String>, content: &str, stat: &str) {
    write_idr_at(path, purpose, content, stat, &now_datetime());
}

fn write_idr_at(path: &Path, purpose: &Option<String>, content: &str, stat: &str, datetime: &str) {
    let purpose_text = purpose.as_deref().unwrap_or("(目的抽出失敗)");

    let body = format!(
        "# IDR: {purpose_text}\n\n\
         > {datetime}\n\n\
         {content}\n\n\
         ---\n\n\
         ### git diff --stat\n\
         ```\n{stat}\n```\n"
    );

    if let Some(parent) = path.parent() {
        create_dir_warn(parent);
    }
    if let Err(e) = fs::write(path, &body) {
        eprintln!(
            "claude-idr: warning: failed to write IDR {}: {}",
            path.display(),
            e
        );
    }
}

fn today_date() -> String {
    let secs = epoch_now();
    let (y, m, d, _, _) = local_datetime(secs);
    format!("{y:04}-{m:02}-{d:02}")
}

fn now_datetime() -> String {
    let secs = epoch_now();
    let (y, m, d, h, min) = local_datetime(secs);
    format!("{y:04}-{m:02}-{d:02} {h:02}:{min:02}")
}

fn epoch_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn local_datetime(epoch_secs: i64) -> (i32, u32, u32, u32, u32) {
    #[cfg(unix)]
    {
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        let time = epoch_secs as libc::time_t;
        unsafe { libc::localtime_r(&time, &mut tm) };
        (
            tm.tm_year + 1900,
            tm.tm_mon as u32 + 1,
            tm.tm_mday as u32,
            tm.tm_hour as u32,
            tm.tm_min as u32,
        )
    }
    #[cfg(not(unix))]
    {
        let (y, m, d) = epoch_to_civil_utc(epoch_secs as u64);
        let day_secs = (epoch_secs as u64) % 86400;
        (
            y,
            m,
            d,
            (day_secs / 3600) as u32,
            ((day_secs % 3600) / 60) as u32,
        )
    }
}

#[cfg(not(unix))]
fn epoch_to_civil_utc(epoch_secs: u64) -> (i32, u32, u32) {
    let z = (epoch_secs / 86400) as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
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

    #[test]
    fn resolve_uses_fixed_output_dir_when_set() {
        let tmp = TempDir::new().unwrap();
        let fixed_dir = tmp.path().join("my-idrs");
        let config = Config {
            output_dir: Some(fixed_dir.clone()),
            workspace_dir: tmp.path().to_path_buf(),
            ..Config::default()
        };

        let result = resolve_with_date(&config, "2026-02-07");

        assert_eq!(result, fixed_dir);
        assert!(result.is_dir());
    }

    #[test]
    fn resolve_fixed_output_dir_takes_priority_over_sow() {
        let tmp = TempDir::new().unwrap();
        let fixed_dir = tmp.path().join("fixed");
        let sow_dir = tmp.path().join("sow-project");
        fs::create_dir_all(&sow_dir).unwrap();
        let sow_file = sow_dir.join("sow.md");
        fs::write(&sow_file, "# SOW").unwrap();
        let current_sow = tmp.path().join(".current-sow");
        fs::write(&current_sow, sow_file.to_str().unwrap()).unwrap();

        let config = Config {
            output_dir: Some(fixed_dir.clone()),
            workspace_dir: tmp.path().to_path_buf(),
            ..Config::default()
        };

        let result = resolve_with_date(&config, "2026-02-07");

        assert_eq!(result, fixed_dir);
    }

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
        assert!(result.is_dir());
    }

    #[test]
    fn resolve_uses_sow_directory_when_valid_current_sow() {
        let tmp = TempDir::new().unwrap();
        let sow_dir = tmp.path().join("sow").join("project-x");
        fs::create_dir_all(&sow_dir).unwrap();
        let sow_file = sow_dir.join("sow.md");
        fs::write(&sow_file, "# SOW").unwrap();
        let current_sow = tmp.path().join(".current-sow");
        fs::write(&current_sow, sow_file.to_str().unwrap()).unwrap();

        let config = Config {
            workspace_dir: tmp.path().to_path_buf(),
            ..Config::default()
        };

        let result = resolve_with_date(&config, "2026-02-07");
        assert_eq!(result, fs::canonicalize(&sow_dir).unwrap());
    }

    #[test]
    fn resolve_falls_back_to_date_when_sow_points_outside_workspace() {
        let workspace = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();

        let outside_file = outside.path().join("sow.md");
        fs::write(&outside_file, "# SOW").unwrap();
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

    #[test]
    fn write_idr_creates_file_with_correct_format() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("idr-01.md");
        let purpose = Some("テスト目的".to_string());
        let content = "## 変更概要\n\nテスト内容";
        let stat = " src/main.rs | 10 +++++++---";

        write_idr_at(&path, &purpose, content, stat, "2026-02-07 14:30");

        let result = fs::read_to_string(&path).unwrap();
        assert!(result.starts_with("# IDR: テスト目的\n\n> 2026-02-07 14:30"));
        assert!(result.contains(content));
        assert!(result.contains("---\n\n### git diff --stat\n```\n"));
        assert!(result.contains(stat));
        assert!(result.ends_with("```\n"));
    }

    #[test]
    fn write_idr_uses_fallback_purpose_when_none() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("idr-01.md");

        write_idr_at(&path, &None, "content", "stat", "2026-01-01 00:00");

        let result = fs::read_to_string(&path).unwrap();
        assert!(result.starts_with("# IDR: (目的抽出失敗)\n\n> 2026-01-01 00:00"));
    }

    #[test]
    fn write_idr_creates_parent_directories() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nested").join("dir").join("idr-01.md");

        write_idr_at(&path, &None, "content", "stat", "2026-01-01 00:00");

        assert!(path.exists());
    }

    #[test]
    fn local_datetime_returns_valid_components() {
        let (y, m, d, h, min) = local_datetime(1770422400); // 2026-02-07 UTC
        assert!(y >= 2026 && y <= 2027);
        assert!((1..=12).contains(&m));
        assert!((1..=31).contains(&d));
        assert!(h < 24);
        assert!(min < 60);
    }

    #[cfg(unix)]
    #[test]
    fn validate_sow_path_rejects_symlink_outside_workspace() {
        let workspace = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();

        let outside_file = outside.path().join("sow.md");
        fs::write(&outside_file, "# SOW").unwrap();

        let link_path = workspace.path().join("sneaky-link.md");
        std::os::unix::fs::symlink(&outside_file, &link_path).unwrap();

        let result = validate_sow_path(&link_path, workspace.path());
        assert!(result.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn local_datetime_epoch_zero_returns_1970() {
        // epoch 0 in any timezone should be 1970-01-01 (or 1969-12-31 for west of UTC)
        let (y, _, _, _, _) = local_datetime(0);
        assert!(y == 1970 || y == 1969);
    }
}
