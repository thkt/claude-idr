# Silent Failure Reviewer Memory

## Project: claude-idr

- Language: Rust
- Architecture: CLI tool with modules: main, claude, config, context, git, jsonl, path, prompt, session
- Error strategy: Mix of `Option<T>` returns with `eprintln!` warnings; no structured logging

## Key Patterns Found

- `jsonl::iter_values` is a critical building block used by session + context modules; its triple `.ok()` chain (File::open, lines, JSON parse) silently swallows all errors
- The codebase generally does log via eprintln before returning None/default, but `jsonl.rs` and `path::next_number` are exceptions
- `.flatten()` on directory iterators (session.rs:57, path.rs:62) is a common Rust idiom but hides entry-level errors

## Rust-Specific Silent Failure Patterns

- `.ok()?` after `.map_err(|e| eprintln!(...))` -- logs then discards; acceptable pattern
- `.ok()?` without preceding `.map_err(...)` -- truly silent, needs flagging
- `.unwrap_or_default()` on `Duration` (path.rs:122) -- epoch time fallback, low risk
- `map_while(Result::ok)` on IO iterators -- stops at first error, drops remainder silently
- `.flatten()` on `ReadDir` iterators -- common but hides individual entry errors

## Review Approach

- Start with `jsonl.rs` or equivalent data-layer modules; errors there cascade
- Check if `.ok()` has a preceding `.map_err()` with logging -- if yes, lower severity
- Trace None/empty returns through call chains to find misleading user-facing messages
