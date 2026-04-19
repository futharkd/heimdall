# Contributing to heimdall

## Prerequisites

- Rust toolchain with support for edition 2024

## Local checks

Run these before opening a merge request:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

## Code organization

- Keep command parsing in `src/cli`.
- Keep orchestration in `src/commands`.
- Keep reusable behavior in `src/modules`.
- Keep output rendering in `src/output`.

## Command design principles

- Prefer non-mutating verification workflows first.
- Keep command handlers thin and delegate to modules.
- Return clear exit codes for automation compatibility.
