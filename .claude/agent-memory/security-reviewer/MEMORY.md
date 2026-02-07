# Security Reviewer Memory

## Project: claude-idr

- Rust CLI tool that generates Implementation Decision Records from git diffs using Claude CLI
- Runs as a git pre-commit hook
- Spawns subprocesses: `claude` CLI (via stdin pipe) and `git` (hardcoded args)
- Config loaded from `~/.config/claude-idr/config.json` (JSON, serde deserialization)
- Session files read from `~/.claude/projects/` (recursive JSONL scan)

## Key Security Patterns Observed

- Uses Rust `Command` API (no shell injection possible)
- XML escaping for prompt injection defense (5 standard entities)
- `canonicalize` + `starts_with` for path traversal defense in SOW resolution
- Shell script uses `set -euo pipefail` and proper quoting

## Sub-threshold Items to Watch

- `config.language` raw value passes into prompt template unescaped (prompt.rs:16 fallback)
- Session dir traversal follows symlinks without depth limit (session.rs:59)
- TOCTOU in validate_sow_path (acknowledged, low-impact)

## Review Checklist for This Codebase

1. Check subprocess args for user-controlled input (currently safe)
2. Check prompt templates for unescaped interpolation (language field is edge case)
3. Check path resolution for traversal (currently mitigated)
4. Check recursive directory walks for symlink loops (OS handles ELOOP)
