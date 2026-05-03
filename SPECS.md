# Heimdall Specifications

## Project overview

`heimdall` is a modular Rust CLI for infrastructure workflows with safety-first execution, clear command boundaries, and CI-enforced quality gates.

- Repository: `https://github.com/futharkd/heimdall`
- Crate: `heimdall`
- Binary: `heimdall`
- Rust edition: `2024`
- Minimum toolchain in repo: `rust-version = 1.94`
- License: MIT

## Goals

- Provide explicit, task-scoped operational commands (`bootstrap`, `harden`, `verify`, `update`).
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
  - Install path: `--install-method binary|package`, env `HEIMDALL_NETBIRD_INSTALL_METHOD`, interactive prompt when neither applies and `--yes`/`--dry-run` are not set; default for non-interactive runs is **binary** (`USE_BIN_INSTALL=true`). **Package** sets `DEBIAN_FRONTEND=noninteractive` for quieter apt.
  - Join uses the official CLI: `netbird up` with optional `--setup-key` and `--management-url` (from flags or `NETBIRD_SETUP_KEY` / `NETBIRD_MANAGEMENT_URL`).
  - Verify runs `netbird status` and requires `Management: Connected` and `Signal: Connected` in output; `ip link show wt0` is best-effort (warning if absent).
  - Dry-run redacts sensitive env vars and `--setup-key` values in reported command lines.
- `heimdall bootstrap k3s`
  - Implemented.
  - Idempotent by default: before planning, probes `command -v k3s`; if found and **`--force` is not set**, the get.k3s.io download and install steps are omitted (still runs verify when not `--skip-start`). **`--force`** always includes download + install in the plan (re-run installer / upgrade path).
  - Delegates install to the official `https://get.k3s.io` script (downloaded with `curl -fsSL` to a temp file, then executed with `sh` and an explicit environment block).
  - `--role server` (default) runs a standard server install. `--role agent` requires `--server-url` / `K3S_URL` (must start with `https://` and include a host) and `--token` / `K3S_TOKEN`.
  - Version pin: `--version` or env `INSTALL_K3S_VERSION` → `INSTALL_K3S_VERSION`. Extra k3s process args: `--install-exec` or env `INSTALL_K3S_EXEC` → `INSTALL_K3S_EXEC`.
  - `--skip-start` sets `INSTALL_K3S_SKIP_START=true`; `--skip-enable` sets `INSTALL_K3S_SKIP_ENABLE=true`. When `--skip-start` is set, the post-install `sudo k3s kubectl get nodes -o name` verification step is omitted (no cluster API check).
  - Verify (when not skipping start): runs `sudo k3s kubectl get nodes -o name` (reads root-owned `/etc/rancher/k3s/k3s.yaml`) and requires at least one `node/...` line in stdout.
  - Dry-run redacts reported env values for keys whose names imply secrets (e.g. `K3S_TOKEN`, any key containing `TOKEN` or `SECRET`, and `GITHUB_TOKEN`).
  - Supports `--dry-run`, `--yes`, `--force`, and `--output human|json`.
- `heimdall update`
  - Implemented (Linux x86_64 only).
  - Resolves GitHub Releases URLs from `CARGO_PKG_REPOSITORY` (layout: `https://github.com/owner/repo/releases/{latest|tag}/heimdall-linux-amd64` plus `.sha256`).
  - Fetches the published `.sha256`, compares it to the SHA256 of the file behind `current_exe()`, and skips the binary download when digests match unless `--force`.
  - `--force` skips only that digest-equality short-circuit; the downloaded artifact is still verified against the remote `.sha256` before `rename`.
  - Uses `curl` subprocesses via `CommandRunner`; optional `GITHUB_TOKEN` is passed as `Authorization: token` header on `curl` and is **redacted** in all reported command strings.
  - Supports `--dry-run`, `--yes`, `--output human|json`, and `--tag` for a non-`latest` package version string.
- `heimdall bootstrap flux`
  - Implemented (SSH deploy key + [`flux bootstrap git`](https://fluxcd.io/flux/cmd/flux_bootstrap_git/); no `--token-auth` / HTTPS PAT in this version).
  - **Idempotency:** `kubectl get ns <namespace>` with `KUBECONFIG`; if the namespace exists, plans `flux reconcile source git flux-system` + `flux reconcile kustomization flux-system` + `flux get kustomization flux-system` (names are fixed to **`flux-system`** for this MVP).
  - **First install:** optional Flux CLI install via `curl` + `bash` on `https://fluxcd.io/install.sh` (override `--install-script-url`) when `flux version` is missing and **`--force` is not set** to skip that probe. `flux bootstrap git` is invoked with **`--silent`** after Heimdall’s deploy-key prompt so Flux skips its duplicate confirmation.
  - **SSH key:** default interactive path runs **`ssh-keygen`** (`ed25519`, empty passphrase), prints the **public** key and GitLab/GitHub deploy-key hints, then waits for Enter before `flux bootstrap git`. Requires a **TTY** unless **`--private-key-file`** supplies an existing private key (BYOK / CI). **`--keep-generated-key <dir>`** copies `deploy_key` + `deploy_key.pub` into that directory after a **successful** bootstrap, then deletes the temp pair; otherwise temp keys are removed (cluster keeps credentials in Kubernetes `Secret`s for ongoing sync).
  - Flags: `--url` (or `FLUX_GIT_URL`), `--branch` / `FLUX_GIT_BRANCH` (default `main`), `--path` / `FLUX_GIT_PATH`, `--namespace` / `FLUX_NAMESPACE` (default `flux-system`), `--kubeconfig` (default `$KUBECONFIG` or `/etc/rancher/k3s/k3s.yaml`), `--private-key-file`, `--private-key-passphrase` (BYOK encrypted keys → Flux `--password`), `--install-script-url`, `--keep-generated-key`, `--force`, `--dry-run`, `--yes`, `--output human|json`.
  - If **`--url` and `FLUX_GIT_URL` are both unset** and **stdin is a TTY**, Heimdall **prompts** for an SSH Git URL (repeats until input validates). **Non-interactive** runs (no TTY) must set **`--url`** or **`FLUX_GIT_URL`**.
  - Same pattern for **`--path` / `FLUX_GIT_PATH`**: interactive prompt when unset and stdin is a TTY; otherwise pass **`--path`** or **`FLUX_GIT_PATH`**.
  - Git URL must be SSH (`ssh://…` or `git@host:path`); `https://` is rejected (would need `--token-auth`, not implemented here). SCP-style `git@host:org/repo.git` is **normalized** to `ssh://git@host/org/repo.git` before `flux bootstrap git` because Flux parses `--url` with Go’s URL parser (which rejects the colon in `host:path`).
  - Deploy keys need **write** access for **bootstrap** because `flux bootstrap git` **pushes** the initial Flux manifests and sync metadata into the repository; read-only keys cannot complete that step. Ongoing reconciliation is mostly reads from Git, but the first bootstrap commit still requires push permission.
  - Dry-run redacts `--private-key-file=…` and `--password` values in planned `flux bootstrap git` command lines.
- `heimdall harden ssh`
  - Placeholder only (returns warning status).

## Architecture

Heimdall now uses a hybrid architecture: feature-first folders for domain logic plus shared cross-cutting layers.

- `src/main.rs`: startup, tracing init, dispatch, process exit.
- `src/cli`: clap command tree and argument models.
- `src/commands`: thin route dispatch only.
- `src/features`: feature-owned command + behavior implementation.
  - `src/features/bootstrap/user`: `input`, `validate`, `plan`, `execute`, `report`, `human`, `command`
  - `src/features/bootstrap/k3s`: `input`, `validate`, `plan`, `execute`, `report`, `human`, `command`
  - `src/features/bootstrap/flux`: `input`, `validate`, `plan`, `execute`, `report`, `human`, `command`, `keygen`
  - `src/features/bootstrap/netbird`: `input`, `validate`, `plan`, `execute`, `report`, `human`, `command`
  - `src/features/verify/doctor`: `checks`, `report`, `human`, `command`
  - `src/features/update`: `package`, `checksum`, `input`, `execute`, `report`, `human`, `command`
- `src/core`: shared execution contracts/types (operation status/results/plans).
- `src/output`: shared `Style` / `--color` / `NO_COLOR` handling (`style.rs`); per-feature human formatting lives under `src/features/.../human.rs`.
- `src/runtime`: exit status and tracing bootstrap.
- `src/runner`: command execution abstraction (`CommandRunner`, `LocalRunner`, `IoMode` with `LiveTee` for live child I/O plus capture).

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

- Global **`--color auto|always|never`** (flattened on the root CLI). **`NO_COLOR`** disables ANSI. **`--output json`** never emits ANSI in printed JSON.
- Human reports: each feature formats via its own `human.rs` using shared `Style` (status tokens and headings colored; long opaque details like digests stay uncolored).
- **`IoMode::LiveTee`**: for human non-dry-run flows, subprocess stdout/stderr are copied to the terminal as data arrives and still accumulated for parsing / final report lines. Dry-run and JSON use buffered capture only.
- JSON output: serialized report structs via `serde`.

## Exit status model

Defined in runtime as:

- `Success` -> `0`
- `Warning` -> `0`
- `Failure` -> `1`

Notes:

- Placeholder commands return warning status.
- Implemented commands return failure on hard failures.

### Update

`heimdall update` mutates the running binary on disk when an update is applied (or when `--force` triggers a reinstall). It requires `curl` on `PATH` and write access to the directory containing the current executable (often `sudo` for `/usr/local/bin` installs).

## Quality gates and CI

GitHub Actions workflow (`.github/workflows/ci.yml`):

- `fmt`: `cargo fmt --check`
- `clippy`: `cargo clippy --all-targets --all-features -- -D warnings`
- `test`: `cargo test --all-targets --all-features`
- `build`: `cargo build --release`, creates `dist/heimdall-linux-amd64` and `.sha256` artifacts
- `release`: uploads binary + checksum to GitHub Releases (on tags matching `v*`)

## Current test coverage

- CLI parse tests:
  - `verify doctor --output json`
  - `bootstrap user` flags parsing
  - `bootstrap netbird` flags parsing
  - `bootstrap k3s` flags parsing (including `--force`)
  - `bootstrap flux` flags parsing
  - `update` flags parsing
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
- `bootstrap k3s` feature tests:
  - plan uses `get.k3s.io`, sets `INSTALL_K3S_*` / agent `K3S_URL` / `K3S_TOKEN` env on the install step, and omits kubectl verify when `--skip-start`
  - plan with `skip_install` skips download/install; `skip_install` + `skip_start` yields empty plan
  - dry-run output redacts `K3S_TOKEN` in env display
  - URL validation for agent server URL (`https://` + host)
  - mocked execute reaches `sudo k3s kubectl` verify when prior steps succeed
- `bootstrap flux` feature tests:
  - SSH URL validation (`ssh://` / `git@…`); `normalize_ssh_git_url_for_flux` / `finalize_flux_git_url` for SCP → `ssh://`; bootstrap plan passes normalized `--url` to Flux
  - `git_url_from_opts_and_env` trims `--url` and returns `None` when flag and env are unset; `cluster_path_from_opts_and_env` for `--path` / `FLUX_GIT_PATH`
  - plan shapes for bootstrap vs reconcile; skip Flux install when `skip_flux_cli_install`
  - dry-run redaction for `flux bootstrap git` `--private-key-file` and `--password`
- `update` feature tests:
  - repository URL parsing and generic package URL construction
  - `sha256sum`-style checksum parsing and SHA256 helpers
  - mocked `curl` ordering (checksum fetch; optional binary fetch when digest differs or `--force`)
  - `--dry-run --force` plans a reinstall without a second `curl` for the binary
  - reported `curl` lines redact `PRIVATE-TOKEN` header values

## Known limitations / next steps

- `harden ssh` is not implemented yet.
- `bootstrap flux` assumes `kubectl`, `flux`, `curl`, `bash`, and (for generated keys) `ssh-keygen` on `PATH`; fixed `flux-system` source/kustomization names; HTTPS PAT bootstrap (`--token-auth`) not implemented.
- `heimdall update` is Linux amd64 only; Windows and macOS are out of scope for v1.
- `bootstrap netbird` assumes a Linux-style host with `curl`, `sh`, `netbird`, and `ip` available on `PATH` after install; it does not configure NetBird management servers.
- `bootstrap k3s` assumes a Linux-style host with `curl`, `sh`, and `k3s` on `PATH` after install for verification; the upstream installer typically requires root. HA servers, air-gapped installs, and non-upstream install methods are out of scope for this command.
  - Idempotent skip is `command -v k3s` only: if `k3s` exists but cluster/agent setup incomplete, use **`--force`** to re-run get.k3s.io.
- `bootstrap user` currently assumes Linux host tools (`sudo`, `getent`, `useradd`, `sed`, `systemctl`, `sshd`).
- Remote execution backend is not implemented yet (local runner only).
- Future module contract can be further formalized into explicit plan/apply/verify traits.
