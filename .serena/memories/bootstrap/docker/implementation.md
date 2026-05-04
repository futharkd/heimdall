# Docker Bootstrap Command Implementation

## Completed Implementation (May 2026)

Added `bootstrap docker` command to heimdall for installing and configuring Docker Engine via the official convenience script (get.docker.com).

### Architecture

Follows the same pattern as existing bootstrap commands (k3s, netbird, flux, user):

- **Module structure**: `src/features/bootstrap/docker/` with 8 modules
  - `input.rs` - CLI opts → DockerConfig + ResolvedDockerInputs struct; env var fallback for DOCKER_INSTALL_SCRIPT_URL; idempotency probe via probe_docker_on_path()
  - `validate.rs` - URL format validation (https/http), username format (alphanumeric + _ + -), registry mirror URL validation
  - `plan.rs` - Enum-based planned operations (Subprocess + WriteFile variants, like komodo)
  - `execute.rs` - Subprocess and file write execution with dry-run support; redaction of sensitive env keys (TOKEN, SECRET, PASSWORD)
  - `report.rs` - BootstrapDockerReport struct with has_failures()
  - `human.rs` - Terminal pretty-print using status_token(label, tone) pattern from k3s/netbird
  - `command.rs` - Orchestration: resolve → probe → build_plan → execute → format output
  - `generate.rs` - serde_json daemon.json generation from log_driver + registry_mirrors

### Operations Generated

1. **install_docker** (conditional): `sh -c "curl -fsSL {url} | sh"` — skipped if docker on PATH (unless --force)
2. **enable_docker_service** (always): `systemctl enable --now docker`
3. **write_daemon_json** (conditional): WriteFile to /etc/docker/daemon.json when log_driver or registry_mirrors set
4. **add_to_docker_group** (conditional): `usermod -aG docker {user}` when --user set
5. **verify_docker** (always): `docker info` with `failure_is_warning: true`

### CLI Flags

- `--install-script-url URL` — default https://get.docker.com (env DOCKER_INSTALL_SCRIPT_URL)
- `--user USERNAME` — add user to docker group
- `--log-driver DRIVER` — written to daemon.json
- `--registry-mirror URL` — repeatable; written to daemon.json
- `--force` — bypass idempotency probe, re-run installer
- `--dry-run` — plan without execution
- `-y / --yes` — skip confirmation prompt
- `--output human|json` — human (default) or JSON report

### Integration Points

- Added `Docker(BootstrapDockerCommand)` variant to `BootstrapAction` enum in cli/mod.rs
- Added CLI struct with proper docstrings and attributes
- Added dispatch arm in commands/mod.rs
- Added `pub mod docker;` to src/features/bootstrap/mod.rs

### Testing

- 126 total tests pass (including new docker tests)
- Tests cover: CLI parsing, input validation, plan shape (skip conditions), daemon.json generation, env redaction, idempotency probe
- All tests use dry_run: true to avoid TTY confirmation issues in test environment

### Code Quality

- cargo fmt ✓
- cargo clippy --all-targets --all-features -- -D warnings ✓
- cargo test --all-targets --all-features ✓

### Notable Design Decisions

1. **Enum-based operations** (like komodo): allows both Subprocess and WriteFile operations in a single plan
2. **Static operation descriptions**: all descriptions are &'static str to avoid lifetime issues; dynamic details go in detail field
3. **No confirmed field**: unlike some input patterns, confirmation is implicit in probe logic + yes/dry_run flags
4. **Idempotency via probe**: checks `command -v docker` before planning install; matches k3s pattern exactly
5. **Redaction in dry-run**: sensitive env keys (TOKEN, SECRET, PASSWORD) redacted in dry-run detail output
6. **Failure is warning only for verify**: docker info can fail if daemon hasn't fully started; non-fatal
