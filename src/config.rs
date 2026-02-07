use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_open_after_generate")]
    #[allow(dead_code)] // Will be used when open-after-generate is implemented
    pub open_after_generate: bool,
    #[serde(default = "default_workspace_dir")]
    pub workspace_dir: PathBuf,
    #[serde(default = "default_session_max_age_min")]
    pub session_max_age_min: u64,
}

fn default_enabled() -> bool {
    true
}
fn default_language() -> String {
    "ja".to_string()
}
fn default_model() -> String {
    "sonnet".to_string()
}
fn default_open_after_generate() -> bool {
    false
}
fn default_workspace_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("workspace")
}
fn default_session_max_age_min() -> u64 {
    30
}

impl Config {
    /// Load config from the given path, or use defaults if no file exists.
    pub fn load(path: Option<&Path>) -> Config {
        let config_path = path.map(PathBuf::from).unwrap_or_else(Self::default_path);

        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Config::default(),
            Err(e) => {
                eprintln!(
                    "claude-idr: warning: cannot read config {}: {}",
                    config_path.display(),
                    e
                );
                return Config::default();
            }
        };

        match serde_json::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!(
                    "claude-idr: warning: invalid config {}: {}",
                    config_path.display(),
                    e
                );
                Config::default()
            }
        }
    }

    fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
            .join("claude-idr")
            .join("config.json")
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: default_enabled(),
            language: default_language(),
            model: default_model(),
            open_after_generate: default_open_after_generate(),
            workspace_dir: default_workspace_dir(),
            session_max_age_min: default_session_max_age_min(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn load_returns_defaults_when_no_config_file() {
        // When no config file exists, Config::load should return default values
        let config = Config::load(None);

        assert!(config.enabled);
        assert_eq!(config.language, "ja");
        assert_eq!(config.model, "sonnet");
        assert_eq!(config.session_max_age_min, 30);
        assert!(!config.open_after_generate);
    }

    #[test]
    fn load_reads_from_config_file() {
        // When a valid config file exists, Config::load should read its values
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"enabled": false, "language": "en", "model": "opus", "session_max_age_min": 60}}"#
        )
        .unwrap();

        let config = Config::load(Some(file.path()));

        assert!(!config.enabled);
        assert_eq!(config.language, "en");
        assert_eq!(config.model, "opus");
        assert_eq!(config.session_max_age_min, 60);
    }

    #[test]
    fn load_with_partial_config_uses_defaults_for_missing_fields() {
        // When config file has only some fields, missing fields should use defaults
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"language": "en"}}"#).unwrap();

        let config = Config::load(Some(file.path()));

        // Explicitly set
        assert_eq!(config.language, "en");
        // Defaults for missing fields
        assert!(config.enabled);
        assert_eq!(config.model, "sonnet");
        assert_eq!(config.session_max_age_min, 30);
    }

    #[test]
    fn load_returns_defaults_for_invalid_json() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{{ invalid json }}").unwrap();
        let config = Config::load(Some(file.path()));
        assert!(config.enabled);
        assert_eq!(config.model, "sonnet");
    }

    #[test]
    fn default_values_are_correct() {
        let config = Config::default();

        assert!(config.enabled);
        assert_eq!(config.language, "ja");
        assert_eq!(config.model, "sonnet");
        assert_eq!(config.session_max_age_min, 30);
        assert!(!config.open_after_generate);
    }
}
