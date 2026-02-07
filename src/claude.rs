use crate::config::Config;
use std::io::Write;
use std::process::{Command, Stdio};

/// Runs a prompt through the claude CLI and returns the output.
/// Returns None on failure (fail-open).
pub fn run(prompt: &str, config: &Config) -> Option<String> {
    let mut child = Command::new("claude")
        .args(build_command(config))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(prompt.as_bytes());
    }

    let output = child.wait_with_output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        eprintln!(
            "claude-idr: claude CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        None
    }
}

/// Escapes XML special characters: &, <, >
pub fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Builds the command-line arguments for invoking the claude CLI.
/// Returns the list of arguments (not including the "claude" binary name).
pub fn build_command(config: &Config) -> Vec<String> {
    vec![
        "-p".to_string(),
        "--model".to_string(),
        config.model.clone(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_xml_escapes_ampersand() {
        assert_eq!(escape_xml("a & b"), "a &amp; b");
    }

    #[test]
    fn escape_xml_escapes_less_than() {
        assert_eq!(escape_xml("a < b"), "a &lt; b");
    }

    #[test]
    fn escape_xml_escapes_greater_than() {
        assert_eq!(escape_xml("a > b"), "a &gt; b");
    }

    #[test]
    fn escape_xml_escapes_all_special_chars() {
        assert_eq!(
            escape_xml("<diff>&changes</diff>"),
            "&lt;diff&gt;&amp;changes&lt;/diff&gt;"
        );
    }

    #[test]
    fn escape_xml_returns_unchanged_for_safe_input() {
        assert_eq!(escape_xml("hello world"), "hello world");
    }

    #[test]
    fn build_command_includes_print_and_model_flags() {
        let config = Config::default();
        let args = build_command(&config);

        assert!(args.contains(&"-p".to_string()), "should include -p flag");
        assert!(
            args.contains(&"--model".to_string()),
            "should include --model flag"
        );
        assert!(
            args.contains(&"sonnet".to_string()),
            "should include the model name from config"
        );
    }

    #[test]
    fn build_command_uses_model_from_config() {
        let mut config = Config::default();
        config.model = "opus".to_string();
        let args = build_command(&config);

        assert!(
            args.contains(&"opus".to_string()),
            "should use the model specified in config"
        );
    }
}
