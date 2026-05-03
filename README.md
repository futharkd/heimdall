# heimdall

Rust CLI for **bootstrap**, **verify**, and **self-update** on Linux (x86_64 for prebuilt binaries and `heimdall update`).

## Commands

| Command | Purpose |
|--------|---------|
| `heimdall verify doctor` | Read-only environment checks |
| `heimdall bootstrap user` | Admin user + SSH `authorized_keys` |
| `heimdall bootstrap netbird` | Official NetBird `install.sh`, join, status checks |
| `heimdall bootstrap k3s` | Official get.k3s.io install, optional verify |
| `heimdall update` | Replace running binary from GitHub Releases |
| `heimdall bootstrap flux` | Flux CLI install (optional), SSH `flux bootstrap git` or reconcile existing install |
| `heimdall harden ssh` | Placeholder (not implemented) |

Full flags, safety behavior, limits: **[SPECS.md](SPECS.md)**.

## Install (Linux x86_64)

```bash
curl -fsSL "https://raw.githubusercontent.com/futharkd/heimdall/main/scripts/install.sh" | sh
```

Specific release (replace `<TAG>`):

```bash
curl -fsSL "https://raw.githubusercontent.com/futharkd/heimdall/main/scripts/install.sh" | env HEIMDALL_VERSION="<TAG>" sh
```

For private releases or rate limits, set **`GITHUB_TOKEN`** before `curl` (same var as `heimdall update`).

## Quick examples

```bash
heimdall verify doctor
heimdall verify doctor --output json
heimdall --color never verify doctor
```

## Global CLI behavior

- **`--color auto|always|never`** — global; place **before** the subcommand. **`NO_COLOR`** disables color.
- **`--output human|json`** — per command where supported; JSON output has no ANSI escapes.
- Human mode without **`--dry-run`**: external commands may **stream** stdout/stderr; dry-run and JSON stay buffered.

## Contributing

Repo layout, `cargo fmt` / clippy / test, CI, and how to extend commands: **[CONTRIBUTING.md](CONTRIBUTING.md)**.
