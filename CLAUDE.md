# Heimdall Project Guide

**Repository**: `https://gitlab.com/futharkd/heimdall`  
**Binary**: `heimdall` (Rust CLI for infrastructure bootstrap, verify, harden, update, reset)  
**Edition**: 2024 | **MSRV**: 1.94 | **License**: MIT

## Overview

`heimdall` is a modular CLI for Linux infrastructure workflows with safety-first design: dry-run support, explicit confirmations for risky operations, structured JSON output, and deterministic exit codes.

Commands:
- `verify doctor` — read-only environment checks
- `bootstrap user` — admin user + SSH keys (idempotent)
- `bootstrap netbird` — NetBird install + join (delegates to official installer)
- `bootstrap k3s` — k3s cluster install (idempotent; probes `command -v k3s` before reinstall)
- `bootstrap flux` — Flux GitOps install + reconcile (SSH-based Git bootstrap, TTY-interactive key gen or BYOK)
- `reset cluster` — reset k3s cluster to initial state
- `update` — self-update binary from GitLab Generic Package Registry (x86_64 Linux only)
- `harden ssh` — placeholder (not implemented)

Global flags: `--color auto|always|never` (before subcommand), `--output json` (per-command).

## Directory Structure

```
src/
├── main.rs                 # startup, tracing init, dispatch, exit
├── cli/mod.rs              # clap command tree + arg models
├── commands/mod.rs         # thin dispatch only
├── core/                   # shared types (Operation, Status, Plan, Report)
├── features/               # feature-first module tree
│   ├── bootstrap/          # bootstrap user/netbird/k3s/flux
│   ├── verify/             # verify doctor
│   ├── update/             # update self
│   └── reset/              # reset cluster
├── runner/mod.rs           # CommandRunner, IoMode (LiveTee for live I/O)
├── runtime/mod.rs          # exit status mapping, tracing setup
└── output/                 # shared Style, --color/NO_COLOR, JSON serialization
```

Feature folders follow a pattern (where relevant):
- `input` — resolve args from flags/env/prompts
- `validate` — check inputs, SSH key formats, URLs
- `plan` — build Operation list (idempotent, dry-run safe)
- `execute` — run operations
- `report` — structured result (status, checks/steps, errors)
- `human` — pretty-print report for terminal output
- `command` — subprocess command building

## Development

### Local checks (required before merge request)

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

### CI Pipeline (.gitlab-ci.yml)

1. **test** — fmt, clippy, test
2. **build** — `cargo build --release` → `dist/heimdall-linux-amd64` + `.sha256`
3. **publish** — upload binary + checksum to GitLab Generic Package Registry (`main` → `latest`, tags → tag name)

### Running locally

```bash
cargo build --release
./target/release/heimdall verify doctor
./target/release/heimdall --help
```

Test a specific feature:

```bash
cargo test bootstrap::user -- --test-threads 1
```

## Key Patterns

### Command execution

`CommandRunner` (local subprocess wrapper) supports:
- **IoMode::LiveTee** — stream child I/O to terminal while capturing (human, non-dry-run)
- **IoMode::Capture** — buffer only (dry-run, JSON output)
- Subprocess exit code checking → `anyhow::Result`
- Env var redaction (auto-redact keys containing `TOKEN`, `SECRET`, `PASSWORD`, etc. in reported command lines)

### Dry-run & idempotency

- Plans are purely declarative (no mutations).
- Idempotent checks (e.g. `command -v k3s`, `kubectl get ns`, `grep -qxF` for SSH keys) skip re-execution.
- Dry-run reports planned operations without execution.
- Risky operations (flag-gated, e.g. `--disable-root-login`) require explicit `--yes` or interactive confirmation.

### Output

- **Human**: colored status tokens + operation sequence, per-feature formatting in `human.rs`
- **JSON**: serialized `Report` struct via `serde`; no ANSI escapes
- Exit codes: `Success` (0), `Warning` (0), `Failure` (1)

### Testing

Test patterns:
- CLI parse tests (clap models)
- Feature unit tests (input validation, plan shapes, redaction)
- Mocked subprocess execution for isolation
- Examples: `bootstrap::user` key validation, `bootstrap::k3s` idempotent skip, `update` checksum parsing

## Important Contracts

### bootstrap user

1. Resolve username, SSH keys from flags/env/prompts
2. Validate username format, SSH key algorithms (`ssh-ed25519`, `ssh-rsa`, `ecdsa-sha2-nistp256`)
3. Plan stages: group/user creation, `.ssh` dir/`authorized_keys` setup, atomic key file promotion, optional hardening
4. Idempotent with `grep -qxF` (skip duplicate keys)
5. Stop-on-failure (abort if any operation fails)
6. Risky operations require `--yes` or interactive confirmation

### bootstrap k3s

- Probes `command -v k3s` before planning install steps; skip unless `--force`
- Agent role requires `--server-url` (SSH URL validation: must start with `https://` + host)
- Env: `INSTALL_K3S_VERSION`, `INSTALL_K3S_EXEC`, `INSTALL_K3S_SKIP_START`, `INSTALL_K3S_SKIP_ENABLE`, `K3S_TOKEN`, `K3S_URL`
- Verify: `sudo k3s kubectl get nodes -o name` (requires at least one `node/…` line)
- Token/URL redacted in dry-run output

### bootstrap flux

- Requires SSH Git URL (`ssh://git@…` or `git@host:org/repo.git`); HTTPS rejected (no `--token-auth` yet)
- SCP-style URLs normalized to `ssh://` for Flux's Go URL parser
- Idempotency: `kubectl get ns <namespace>`; if exists, `flux reconcile source git flux-system` + `flux reconcile kustomization flux-system`
- SSH key generation interactive by default (TTY required; `ed25519`, empty passphrase) or BYOK via `--private-key-file`
- Deploy keys need **write** access (Flux pushes manifests on first bootstrap)
- Optional Flux CLI auto-install via `https://fluxcd.io/install.sh` (skip with `--force`)
- Flags: `--url` / `FLUX_GIT_URL`, `--branch` / `FLUX_GIT_BRANCH` (default `main`), `--path` / `FLUX_GIT_PATH`, `--namespace` / `FLUX_NAMESPACE` (default `flux-system`), `--kubeconfig`, `--private-key-file`, `--private-key-passphrase`, `--keep-generated-key <dir>`, `--install-script-url`, `--force`, `--dry-run`, `--yes`, `--output json`

### update

- Fetches `.sha256` from GitLab Generic Package (`{latest|tag}/heimdall-linux-amd64.sha256`)
- Compares to SHA256 of current binary; skips download if match (unless `--force`)
- `--force` skips digest short-circuit but still verifies downloaded artifact
- Uses `curl` with optional `GITLAB_TOKEN` / `PRIVATE_TOKEN` (redacted in output)
- Replaces running binary on disk (requires write access to exe directory)

## Extending

### Adding a new bootstrap command

1. Create `src/features/bootstrap/<feature>/` with modules following the pattern (input, validate, plan, execute, report, human, command)
2. Define input struct in CLI (`src/cli/mod.rs`)
3. Add command handler in `src/commands/mod.rs` that calls feature (resolve input → build plan → execute → format report)
4. Add tests in the feature folder
5. Update `SPECS.md` with command spec and flags
6. Update `README.md` with command summary

### Common mistakes

- **Mutations in verify paths**: Keep verification non-mutating; mutation belongs in `bootstrap` or explicit `--apply` flag
- **Hardcoding paths/commands**: Use `which` or `command -v` to locate executables
- **Skipping idempotency checks**: Always check if a resource exists before creating (e.g. `command -v k3s`, `kubectl get ns`)
- **Not redacting secrets in output**: Add keys to redaction logic when reporting commands with sensitive env vars
- **Mixing business logic into handlers**: Keep `src/commands` thin; put logic in feature modules

## References

- **[SPECS.md](SPECS.md)** — canonical command specs, flags, safety behavior, known limitations
- **[README.md](README.md)** — user-facing quick start
- **[CONTRIBUTING.md](CONTRIBUTING.md)** — contribution workflow, code organization, design notes
- **[.gitlab-ci.yml](.gitlab-ci.yml)** — CI job definitions
- **[scripts/install.sh](scripts/install.sh)** — distribution install script

## Notable recent work

- **Feb 2025**: Flux bootstrap SSH key generation, deploy key write requirement, idempotent reconcile
- **Apr 2025**: k3s idempotency probe (`command -v`), `--force` flag; Flux default branch auto-detect; reset cluster command added
- **Latest**: Clippy warnings fixed (2024 edition)

## Quick command reference

```bash
# Development
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test

# Build
cargo build --release

# Test a single feature
cargo test bootstrap::flux --lib

# Try a command
./target/release/heimdall verify doctor --output json
./target/release/heimdall bootstrap user --help
```
