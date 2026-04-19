# Heimdall Specifications

## Project overview

`heimdall` is a modular Rust CLI for infrastructure workflows with safety-first execution, clear command boundaries, and CI-enforced quality gates.

- Repository: `https://gitlab.com/futharkd/heimdall`
- Crate: `heimdall`
- Binary: `heimdall`
- Rust edition: `2024`
- Minimum toolchain in repo: `rust-version = 1.94`
- License: MIT

## Goals

- Provide explicit, task-scoped operational commands (`bootstrap`, `harden`, `verify`).
- Keep command handlers thin and push behavior into reusable modules.
- Prefer safe defaults:
  - non-mutating verification commands
  - dry-run support
  - explicit confirmation for risky mutations
  - deterministic output and exit codes for automation

## Current command surface

- `heimdall verify doctor`
  - Implemented.
  - Performs local, non-mutating environment checks.
  - Supports `--output human|json`.
- `heimdall bootstrap user`
  - Implemented.
  - Creates/ensures admin user and group, provisions SSH keys, and can apply guarded SSH auth hardening.
  - Supports interactive prompts for missing input.
  - Supports `--dry-run`, `--yes`, and `--output human|json`.
- `heimdall bootstrap netbird`
  - Implemented.
  - Delegates install to the official `https://pkgs.netbird.io/install.sh` (downloaded to a temp file, then executed with `NETBIRD_RELEASE` / optional `SKIP_UI_APP` / optional `GITHUB_TOKEN` from the environment).
  - Join uses the official CLI: `netbird up` with optional `--setup-key` and `--management-url` (from flags or `NETBIRD_SETUP_KEY` / `NETBIRD_MANAGEMENT_URL`).
  - Verify runs `netbird status` and requires `Management: Connected` and `Signal: Connected` in output; `ip link show wt0` is best-effort (warning if absent).
  - Dry-run redacts sensitive env vars and `--setup-key` values in reported command lines.
- `heimdall bootstrap flux`
  - Placeholder only (returns warning status).
- `heimdall harden ssh`
  - Placeholder only (returns warning status).

## Architecture

Heimdall now uses a hybrid architecture: feature-first folders for domain logic plus shared cross-cutting layers.

- `src/main.rs`: startup, tracing init, dispatch, process exit.
- `src/cli`: clap command tree and argument models.
- `src/commands`: thin route dispatch only.
- `src/features`: feature-owned command + behavior implementation.
  - `src/features/bootstrap/user`: `input`, `validate`, `plan`, `execute`, `report`, `command`
  - `src/features/bootstrap/netbird`: `input`, `validate`, `plan`, `execute`, `report`, `command`
  - `src/features/verify/doctor`: `checks`, `report`, `command`
- `src/core`: shared execution contracts/types (operation status/results/plans).
- `src/output`: human-readable output formatting.
- `src/runtime`: exit status and tracing bootstrap.
- `src/runner`: command execution abstraction (`CommandRunner`, `LocalRunner`).

## Implemented workflows

### Verify doctor

`verify doctor` executes read-only checks and emits a structured report:

- `cargo` availability check
- current working directory readability
- `.git` presence warning/pass behavior

Exit behavior:

- failure check present => exit code `1`
- no failures => exit code `0`

### Bootstrap user

`bootstrap user` resolves inputs from flags first, then prompts as needed:

- prompts for username if `--user` missing
- prompts for allowed SSH public keys if neither `--key` nor `--key-file` provided
- prompts for explicit confirmation if risky flags are requested:
  - `--disable-root-login`
  - `--disable-password-auth`
  - unless `--yes` is set

Validation:

- username format validation
- SSH key shape and allowed algorithm validation (`ssh-ed25519`, `ssh-rsa`, `ecdsa-sha2-nistp256`)
- requires at least one valid key

Planned operation stages:

1. Ensure group exists (idempotent)
2. Ensure user exists (idempotent)
3. Ensure `.ssh` directory ownership and permissions
4. Ensure `authorized_keys` exists
5. Prepare temp key file copy (`.authorized_keys.tmp`)
6. Append missing keys into temp file only (idempotent with `grep -qxF`)
7. Atomically promote temp file to `authorized_keys`
8. Set ownership and mode on `authorized_keys`
9. Optional hardening updates to `/etc/ssh/sshd_config` (guarded)
10. Validate sshd config and reload service when hardening applied

Safety behavior:

- risky operations are marked confirmation-required
- unconfirmed risky operations are skipped and reported
- on command failure, execution stops and returns failure
- dry-run reports planned operations without execution

## Output model

- Human output:
  - `verify doctor`: pass/warn/fail lines
  - `bootstrap user`: per-operation plan/skip/ok/fail lines
- JSON output:
  - serialized report structs via `serde`

## Exit status model

Defined in runtime as:

- `Success` -> `0`
- `Warning` -> `0`
- `Failure` -> `1`

Notes:

- Placeholder commands return warning status.
- Implemented commands return failure on hard failures.

## Quality gates and CI

GitLab CI stages:

- `test`:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-targets --all-features`
- `build`:
  - `cargo build --release`
  - publishes `dist/heimdall-linux-amd64` and `.sha256` as job artifacts
- `publish`:
  - uploads binary + checksum to GitLab Generic Package Registry
  - `main` branch => `heimdall/latest`
  - tags => `heimdall/<tag>`

## Current test coverage

- CLI parse tests:
  - `verify doctor --output json`
  - `bootstrap user` flags parsing
  - `bootstrap netbird` flags parsing
- `bootstrap user` feature tests:
  - invalid key rejection
  - missing key plan failure
  - idempotent plan semantics for user/key operations
  - stop-on-failure behavior
  - confirmation-gated risky step skipping
- `verify doctor` feature tests:
  - failure detection in report
- `bootstrap netbird` feature tests:
  - plan uses official install URL and passes `NETBIRD_RELEASE` / `SKIP_UI_APP` into the install step
  - dry-run output redacts `GITHUB_TOKEN` and `--setup-key` arguments
  - `netbird status` output parsing for connected management/signal lines

## Known limitations / next steps

- `bootstrap flux` and `harden ssh` are not implemented yet.
- `bootstrap netbird` assumes a Linux-style host with `curl`, `sh`, `netbird`, and `ip` available on `PATH` after install; it does not configure NetBird management servers.
- `bootstrap user` currently assumes Linux host tools (`sudo`, `getent`, `useradd`, `sed`, `systemctl`, `sshd`).
- Remote execution backend is not implemented yet (local runner only).
- Future module contract can be further formalized into explicit plan/apply/verify traits.
