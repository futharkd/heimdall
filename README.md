# futharkd/heimdall

`heimdall` is a modular Rust CLI for infrastructure workflows. The project is structured for safe, incremental growth with stable command namespaces and reusable module boundaries.

## Current command surface

- `heimdall verify doctor`: run non-mutating environment readiness checks.
- `heimdall bootstrap flux`: scaffolded placeholder.
- `heimdall bootstrap user`: scaffolded placeholder.
- `heimdall harden ssh`: scaffolded placeholder.

## Architecture

The codebase is organized by responsibility:

- `src/cli`: clap models and subcommand tree
- `src/commands`: command handlers and top-level dispatch
- `src/modules`: reusable domain modules (currently `doctor`)
- `src/runtime`: initialization and exit status conventions
- `src/output`: human and machine output formatting helpers
- `src/runner`: command-runner abstractions

## Quick start

Requirements:

- Rust toolchain with edition 2024 support (`rustc >= 1.85`)

Run locally:

```bash
cargo run -- verify doctor
```

JSON output:

```bash
cargo run -- verify doctor --output json
```

## Development workflow

Format:

```bash
cargo fmt --check
```

Lint:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Test:

```bash
cargo test --all-targets --all-features
```

## Roadmap

- Implement bootstrap and hardening modules behind the scaffolded command tree.
- Add plan/apply/verify lifecycle contracts per module.
- Extend execution backends for local and remote host operations.
