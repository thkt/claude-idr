# Type Safety Reviewer Memory

## Project: claude-idr

- Language: Rust (edition 2024)
- Dependencies: libc, serde, serde_json, dirs
- Pattern: CLI tool with libc FFI for datetime, subprocess calls for git/claude

## Rust-Specific Review Adaptations

- `as` casts are the primary type-narrowing risk (equivalent to TS `as` assertions)
- `unsafe` blocks replace the role of `any` -- they are where type guarantees break
- FFI boundaries (libc) need special attention: null returns, C struct validity
- Integer overflow: debug panics vs release wrapping -- both problematic
- `from_utf8_lossy` is Rust's equivalent of lossy type coercion

## Key Patterns Found in This Codebase

- `.map_err(|e| eprintln!(...)).ok()?` for error handling -- clean pattern
- `unwrap_or_default()` used safely for Duration (returns zero, not panic)
- Config uses serde defaults -- validation gap for extreme values
- `next_number()` has u32 overflow edge case (max+1)

## Common Rust Type Safety Checklist

1. `as` casts: check for truncation (u64->i64, i64->u64, i32->u32)
2. `unsafe` blocks: verify all C API contracts (null returns, valid inputs)
3. `from_utf8_lossy`: document why lossy is acceptable
4. Serde deserialization: validate ranges post-deserialize
5. Arithmetic: check for overflow in release mode (wrapping vs panicking)
6. `mem::zeroed()`: prefer `MaybeUninit` for FFI structs
