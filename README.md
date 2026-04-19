# futharkd/heimdall

`heimdall` is a modular Rust CLI for infrastructure workflows. The project is structured for safe, incremental growth with stable command namespaces and reusable module boundaries.

## Current command surface

- `heimdall verify doctor`: run non-mutating environment readiness checks.
- `heimdall bootstrap flux`: scaffolded placeholder.
- `heimdall bootstrap user`: create/update admin user and allowed SSH keys.
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

Bootstrap user (non-interactive):

```bash
cargo run -- bootstrap user \
  --user admin \
  --group admin \
  --key-file ~/.ssh/id_ed25519.pub \
  --dry-run
```

Bootstrap user (interactive key prompt):

```bash
cargo run -- bootstrap user --user admin
```

When risky auth changes are requested (`--disable-root-login` and/or `--disable-password-auth`), Heimdall prompts for explicit confirmation unless `--yes` is provided.

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

## Binary distribution (GitLab CI)

CI builds `heimdall-linux-amd64` and publishes it to the GitLab Generic Package Registry.

Main branch publishes a rolling `latest` package:

```bash
wget "https://gitlab.com/api/v4/projects/<PROJECT_ID>/packages/generic/heimdall/latest/heimdall-linux-amd64" -O heimdall
chmod +x heimdall
./heimdall verify doctor
```

Tagged releases publish a versioned package (`<TAG>`):

```bash
wget "https://gitlab.com/api/v4/projects/<PROJECT_ID>/packages/generic/heimdall/<TAG>/heimdall-linux-amd64" -O heimdall
```

Checksum file is published alongside the binary as `heimdall-linux-amd64.sha256`.

## Roadmap

- Implement bootstrap and hardening modules behind the scaffolded command tree.
- Add plan/apply/verify lifecycle contracts per module.
- Extend execution backends for local and remote host operations.
