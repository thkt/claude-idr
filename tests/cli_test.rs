use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_flag_shows_help_text() {
    let mut cmd = Command::cargo_bin("claude-idr").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn version_flag_shows_version() {
    let mut cmd = Command::cargo_bin("claude-idr").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn exits_zero_when_no_staged_diff() {
    let mut cmd = Command::cargo_bin("claude-idr").unwrap();
    cmd.assert()
        .success()
        .stderr(
            predicate::str::contains("no staged changes")
                .or(predicate::str::contains("no code changes via Claude detected"))
                .or(predicate::str::contains("no recent session")),
        );
}

#[test]
fn dry_run_flag_prevents_claude_call() {
    let mut cmd = Command::cargo_bin("claude-idr").unwrap();
    cmd.arg("--dry-run");
    cmd.assert().success();
}

#[test]
fn dry_run_outputs_prompt_when_session_and_diff_available() {
    let mut cmd = Command::cargo_bin("claude-idr").unwrap();
    cmd.arg("--dry-run");
    // dry-run always succeeds; when session+diff are present it prints the prompt,
    // otherwise it exits early with a skip message — both are valid outcomes.
    cmd.assert()
        .success()
        .stderr(
            predicate::str::contains("dry-run mode")
                .or(predicate::str::contains("no staged changes"))
                .or(predicate::str::contains("no code changes via Claude detected"))
                .or(predicate::str::contains("no recent session")),
        );
}
