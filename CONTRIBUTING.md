# Contributing to heimdall

## Prerequisites

- Rust **1.94** or newer (`rust-version` in [`Cargo.toml`](Cargo.toml); edition **2024**)

## Local checks

CI runs the same gates. Run before opening a merge request:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

See [`.github/workflows/ci.yml`](.github/workflows/ci.yml) for the exact job definitions (fmt, clippy, test, release build).

## Releases and CI

- **Build**: CI produces `heimdall-linux-amd64` + `.sha256` and uploads them to **GitHub Releases** (on `v*` tags).
- **Install script**: [`scripts/install.sh`](scripts/install.sh) — the README `curl … | sh` one-liners wrap this.

## Where code lives

- **`src/cli`** — clap models and the subcommand tree
- **`src/commands`** — thin dispatch only; no heavy logic here
- **`src/features`** — behavior per domain (`bootstrap/*`, `verify/*`, `update`, …). New bootstrap flows usually mirror an existing folder: `input`, `validate`, `plan`, `execute`, `report`, `human`, `command`
- **`src/core`** — shared operation / report types used by plans
- **`src/runner`** — `CommandRunner` and subprocess I/O (`IoMode::LiveTee`, etc.)
- **`src/runtime`** — tracing bootstrap and exit status mapping
- **`src/output`** — shared styling (`--color`, `NO_COLOR`); per-feature human formatting stays under `src/features/.../human.rs`

## Specs and README

- **[`SPECS.md`](SPECS.md)** — canonical description of commands, flags, safety behavior, and known limitations. Update it when behavior or contracts change.
- **[`README.md`](README.md)** — user-facing quick start; keep it aligned with shipped commands when you add or change the CLI surface.

## Design notes

- Prefer **non-mutating** verify paths; mutating flows should support **`--dry-run`** and clear confirmation where appropriate.
- Keep **command handlers thin**; resolve inputs → build plan → execute → format report.
- Use **deterministic exit codes** and structured **`--output json`** where the feature already exposes reports (match existing features).

## Rough roadmap

- Implement scaffolded commands (`harden ssh`, …).
- Tighter plan/apply/verify contracts per feature where it pays off.
- Optional remote execution backend (today everything is local `CommandRunner`).
