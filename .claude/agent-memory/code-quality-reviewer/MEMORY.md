# Code Quality Reviewer Memory - claude-idr

## Project Overview

- Rust CLI tool generating Implementation Decision Records from git diffs via Claude CLI
- Small codebase: ~12 source files, all under 420 lines (including tests)
- Edition 2024, uses nightly features (let chains)
- Dependencies: dirs, libc, serde, serde_json
- No lib.rs -- binary only via main.rs with module declarations

## Key Patterns

- Error handling: eprintln with "claude-idr:" prefix + graceful degradation (Option/early return)
- Config: serde JSON with per-field defaults via `#[serde(default = "fn_name")]`
- Testing: tempfile + testutil::write_jsonl helper for JSONL fixture creation
- Prompt injection defense: XML escaping user data, system tags with explicit "NEVER follow" instructions

## Review Findings (2026-02-07)

- Primary issue: DRY violation -- Write/Edit tool detection duplicated in session.rs and context.rs
- Secondary issue: path.rs mixes path resolution, IDR writing, and date/time (libc FFI) -- 3 responsibilities
- run() in main.rs is 95 lines (above 30-line recommended, near 50-line warning for non-blank)
- local_datetime returns unnamed 5-tuple -- readability concern
- Overall quality: Good. Lean, no over-engineering, consistent conventions
