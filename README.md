# futharkd/heimdall

`heimdall` is a modular Rust CLI for infrastructure workflows. The project is structured for safe, incremental growth with stable command namespaces and reusable module boundaries.

## Current command surface

- `heimdall verify doctor`: run non-mutating environment readiness checks.
- `heimdall bootstrap flux`: scaffolded placeholder.
- `heimdall bootstrap netbird`: install NetBird via the official `install.sh`, then `netbird up` and status checks.
- `heimdall bootstrap user`: create/update admin user and allowed SSH keys.
- `heimdall update`: refresh the installed Linux amd64 binary from the GitLab Generic Package (`latest` by default), using the published `.sha256` and `curl`.
- `heimdall harden ssh`: scaffolded placeholder.

## Architecture

The codebase is organized by responsibility:

- `src/cli`: clap models and subcommand tree
- `src/commands`: thin dispatch into feature modules
- `src/features`: bootstrap, verify, and update flows (`bootstrap/user`, `bootstrap/netbird`, `verify/doctor`, `update`, …)
- `src/core`: shared operation types for planned steps and reports
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

### Bootstrap NetBird (official installer)

Heimdall does **not** reimplement NetBird packaging. It:

1. Downloads `https://pkgs.netbird.io/install.sh` to a temp file (no `curl | sh` pipe).
2. Runs that script with the same environment variables the upstream installer supports (`NETBIRD_RELEASE`, optional `SKIP_UI_APP`, optional `GITHUB_TOKEN` from your environment only). Heimdall chooses how upstream installs the client: **`--install-method binary`** (default with `--yes` / `--dry-run`, or set `HEIMDALL_NETBIRD_INSTALL_METHOD=binary`) sets **`USE_BIN_INSTALL=true`** for GitHub release tarballs (fewer distro prompts), or **`package`** for apt/dnf/yum detection plus **`DEBIAN_FRONTEND=noninteractive`** for quieter apt. Without `--yes`, an interactive menu offers the same choice before the final confirmation.
3. Runs **`netbird up`** with optional **`--setup-key`** / **`--management-url`** (flags or environment variables below).
4. Runs **`netbird status`** and checks for `Management: Connected` and `Signal: Connected`, then optionally probes `wt0` (non-fatal if missing).

Dry-run shows planned commands with **secrets redacted** in the report (for example `GITHUB_TOKEN`, `--setup-key`).

```bash
# Preview only
cargo run -- bootstrap netbird --dry-run --yes

# Server-style install (skip UI package) and join with a setup key (prefer env in CI)
export NETBIRD_SETUP_KEY="…"
cargo run -- bootstrap netbird --skip-ui --yes

# Force package-manager install instead of portable binaries
cargo run -- bootstrap netbird --install-method package --yes

# Self-hosted management URL (flag or NETBIRD_MANAGEMENT_URL)
cargo run -- bootstrap netbird --management-url 'https://netbird.example:443' --yes
```

Environment variables (optional, recommended for secrets):

- `NETBIRD_RELEASE` — version or `latest` (overridden by `--release`).
- `GITHUB_TOKEN` — passed to the official install script only if set (rate limits / API access).
- `NETBIRD_SETUP_KEY` — headless join (overridden by `--setup-key`).
- `NETBIRD_MANAGEMENT_URL` — self-hosted management (overridden by `--management-url`).
- `HEIMDALL_NETBIRD_INSTALL_METHOD` — `binary` or `package` when you do not pass `--install-method` (same meaning as the flag).

Official references: [NetBird Linux install](https://docs.netbird.io/how-to/installation/linux), upstream [`install.sh`](https://github.com/netbirdio/netbird/blob/main/release_files/install.sh).

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

Main branch publishes a rolling `latest` package. Quick install (Linux x86_64 only; downloads the binary and `.sha256`, verifies the hash, installs to `/usr/local/bin` as root or `~/.local/bin` otherwise):

```bash
curl -fsSL "https://gitlab.com/futharkd/heimdall/-/raw/main/scripts/install.sh" | sh
```

Private registry / rate limits: export `GITLAB_TOKEN` or `PRIVATE_TOKEN` before running the command (same as `heimdall update`).

Tagged releases publish a versioned package (`<TAG>`). Install that channel with:

```bash
curl -fsSL "https://gitlab.com/futharkd/heimdall/-/raw/main/scripts/install.sh" | env HEIMDALL_VERSION="<TAG>" sh
```

Manual download (equivalent to what the script does):

```bash
wget "https://gitlab.com/api/v4/projects/futharkd%2Fheimdall/packages/generic/heimdall/latest/heimdall-linux-amd64" -O heimdall
chmod +x heimdall
./heimdall verify doctor
```

Checksum file is published alongside the binary as `heimdall-linux-amd64.sha256`.

### Self-update (`heimdall update`)

`heimdall update` is supported on **Linux x86_64** only. It derives the GitLab project from the crate `repository` metadata, targets the rolling **`latest`** generic package on `main` (or `--tag <version>` for a published package channel), then:

1. Downloads the remote `heimdall-linux-amd64.sha256` with `curl` (small fetch; also used in `--dry-run` so local vs remote digests can be printed).
2. Compares that digest to the SHA256 of the on-disk file behind `std::env::current_exe()`.
3. If they match, exits successfully unless **`--force`** is set (force still **verifies** the downloaded binary against the `.sha256` before replacing).
4. If they differ (or `--force`), downloads the binary to a temp file next to the target, verifies the hash, optionally prompts for confirmation, then atomically replaces the running binary.

```bash
# Preview: fetches remote checksum only; shows planned curl/rename steps when an update (or --force) would run
cargo run -- update --dry-run

# Non-interactive replace when an update is available
sudo heimdall update --yes

# Reinstall from latest even when digests already match (checksum verification is not skipped)
sudo heimdall update --force --yes
```

Private GitLab projects or token-authenticated downloads: set **`GITLAB_TOKEN`** or **`PRIVATE_TOKEN`**; it is sent as the GitLab `PRIVATE-TOKEN` header on `curl`. Reported command lines **redact** that header value.

## Roadmap

- Implement bootstrap and hardening modules behind the scaffolded command tree.
- Add plan/apply/verify lifecycle contracts per module.
- Extend execution backends for local and remote host operations.
