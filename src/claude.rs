use crate::config::Config;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn run(prompt: &str, config: &Config) -> Option<String> {
    let mut child = Command::new("claude")
        .args(build_command(config))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| eprintln!("claude-idr: cannot start claude CLI: {e}"))
        .ok()?;

    if let Some(mut stdin) = child.stdin.take()
        && let Err(e) = stdin.write_all(prompt.as_bytes())
    {
        eprintln!("claude-idr: warning: failed to write prompt: {e}");
        if let Err(e) = child.kill() {
            eprintln!("claude-idr: warning: failed to kill claude process: {e}");
        }
        if let Err(e) = child.wait() {
            eprintln!("claude-idr: warning: failed to wait for claude process: {e}");
        }
        return None;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| eprintln!("claude-idr: warning: failed to wait for claude CLI: {e}"))
        .ok()?;
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

fn build_command(config: &Config) -> Vec<String> {
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
    fn build_command_uses_model_from_config() {
        let mut config = Config::default();
        config.model = "opus".to_string();
        let args = build_command(&config);

        assert_eq!(args, vec!["-p", "--model", "opus"]);
    }
}
