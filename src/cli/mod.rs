use clap::{Parser, Subcommand, ValueEnum};

use crate::output::ColorArg;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
}

/// k3s role for the official `get.k3s.io` install script (via `K3S_URL` / `K3S_TOKEN` for agents).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum K3sRole {
    #[default]
    Server,
    Agent,
}

/// How the official NetBird `install.sh` should install the client (via environment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum NetbirdInstallMethod {
    /// Set `USE_BIN_INSTALL=true` for upstream: GitHub release tarballs, fewer distro/package prompts.
    #[default]
    Binary,
    /// Let upstream pick apt, dnf, or yum. Heimdall sets `DEBIAN_FRONTEND=noninteractive` for quieter apt.
    Package,
}

/// Komodo deployment mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum KomodoMode {
    /// Full stack: MongoDB + Komodo Core + Komodo Periphery.
    #[default]
    Core,
    /// Periphery only: connect to existing remote Core.
    Periphery,
}

impl std::fmt::Display for KomodoMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KomodoMode::Core => write!(f, "Core"),
            KomodoMode::Periphery => write!(f, "Periphery"),
        }
    }
}

#[derive(Debug, Parser)]
pub struct GlobalOpts {
    /// When to emit ANSI colors in human reports (`NO_COLOR` in the environment always disables).
    #[arg(long, value_enum, default_value_t = ColorArg::Auto, global = true)]
    pub color: ColorArg,
}

#[derive(Debug, Parser)]
#[command(name = "heimdall", version, about = "Modular infrastructure CLI")]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Workflow: install and configure infrastructure components (k3s, Flux, NetBird, user + SSH).
    Bootstrap(Box<BootstrapCommand>),
    /// Workflow: harden infrastructure security (SSH config, firewall, etc.).
    Harden(HardenCommand),
    /// Workflow: reset infrastructure to initial state (k3s cluster wipe).
    Reset(ResetCommand),
    /// Workflow: verify infrastructure health and configuration (read-only checks).
    Verify(VerifyCommand),
    /// Workflow: replace the running heimdall binary with a newer version from the package registry.
    Update(UpdateCommand),
}

#[derive(Debug, Subcommand)]
pub enum BootstrapAction {
    /// Install and configure Docker Engine on the current system.
    Docker(BootstrapDockerCommand),
    /// Install Flux GitOps framework with SSH-based Git bootstrap or reconcile existing install.
    Flux(BootstrapFluxCommand),
    /// Install Infisical CLI and configure Universal Auth agent for secrets management.
    Infisical(BootstrapInfisicalCommand),
    /// Install k3s Kubernetes cluster (server or agent) via the official get.k3s.io installer.
    K3s(BootstrapK3sCommand),
    /// Install and join NetBird VPN overlay network via official installer.
    Netbird(BootstrapNetbirdCommand),
    /// Create or update admin user with SSH public key authentication.
    User(BootstrapUserCommand),
    /// Initialize and start Komodo (core or periphery) via Docker Compose.
    Komodo(BootstrapKomodoCommand),
}

#[derive(Debug, clap::Args)]
pub struct BootstrapCommand {
    #[command(subcommand)]
    pub action: BootstrapAction,
}

#[derive(Debug, Subcommand)]
pub enum ResetAction {
    /// Destructively reset k3s cluster to initial state (drain nodes, purge data, uninstall).
    Cluster(ResetClusterCommand),
}

#[derive(Debug, clap::Args)]
pub struct ResetCommand {
    #[command(subcommand)]
    pub action: ResetAction,
}

#[derive(Debug, clap::Args)]
#[command(about = "Destructively reset k3s cluster to initial state.")]
#[command(
    long_about = "Drain all nodes, purge all data, and uninstall k3s. DESTRUCTIVE OPERATION. \
Requires explicit confirmation via --confirm=reset-cluster (or TTY prompt). \
Supports --dry-run (print planned commands without execution), --yes (skip prompt, still requires --confirm). \
Outputs structured report (human or --output json)."
)]
pub struct ResetClusterCommand {
    /// Skip destructive execution and print the planned reset commands only.
    #[arg(long)]
    pub dry_run: bool,
    /// Skip the yes/no prompt; still requires typed destructive confirmation via --confirm (or TTY prompt).
    #[arg(long)]
    pub yes: bool,
    /// Destructive confirmation token. Must equal `reset-cluster`.
    #[arg(long)]
    pub confirm: Option<String>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}

#[derive(Debug, clap::Args)]
#[command(
    about = "Install Flux GitOps framework and bootstrap a Git-based declarative configuration repo."
)]
#[command(
    long_about = "Install Flux and run `flux bootstrap git` against a Git repository (SSH-based deploy key or BYOK). \
Idempotent: if the Flux namespace exists, reconciles existing install instead. \
Requires SSH Git URL (SCP-style git@host:path normalized to ssh://). \
Deploy keys need write access (Flux pushes initial manifests). \
Interactive key generation by default (TTY + ssh-keygen); BYOK via --private-key-file. \
Optional Flux CLI auto-install via https://fluxcd.io/install.sh (skip with --force)."
)]
pub struct BootstrapFluxCommand {
    /// SSH Git URL (e.g. `ssh://git@gitlab.com/group/repo.git`). Omit to use `FLUX_GIT_URL` or an interactive prompt when stdin is a TTY.
    #[arg(long)]
    pub url: Option<String>,
    /// Git branch (default `main`; env `FLUX_GIT_BRANCH`).
    #[arg(long)]
    pub branch: Option<String>,
    /// Path inside the repo for Flux manifests (e.g. `clusters/prod`). Omit to use `FLUX_GIT_PATH` or an interactive prompt when stdin is a TTY.
    #[arg(long)]
    pub path: Option<String>,
    /// Flux namespace (default `flux-system`).
    #[arg(long)]
    pub namespace: Option<String>,
    /// kubeconfig path (default `$KUBECONFIG` or `/etc/rancher/k3s/k3s.yaml`).
    #[arg(long)]
    pub kubeconfig: Option<String>,
    /// Use this SSH private key (deploy key already on server); skips keygen and deploy-key prompt.
    #[arg(long)]
    pub private_key_file: Option<String>,
    /// Passphrase for `--private-key-file` when encrypted (passed to `flux` as `-p`).
    #[arg(long = "private-key-passphrase")]
    pub private_key_passphrase: Option<String>,
    /// Flux install script URL (default upstream install script).
    #[arg(long)]
    pub install_script_url: Option<String>,
    /// After bootstrap, copy generated key material to this directory (`deploy_key` + `deploy_key.pub`).
    #[arg(long)]
    pub keep_generated_key: Option<String>,
    /// Re-run Flux install script even when `flux` is on PATH.
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub yes: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}

#[derive(Debug, clap::Args)]
#[command(about = "Install and configure Docker Engine on the current system.")]
#[command(
    long_about = "Install Docker via the official convenience script (get.docker.com), enable the systemd service, \
optionally write /etc/docker/daemon.json (log driver, registry mirrors), optionally add a user to the docker group, \
and verify the daemon. \
Idempotent: probes `command -v docker` and skips download/install if found (use --force to override). \
Supports --dry-run, --yes, --force, --output json."
)]
pub struct BootstrapDockerCommand {
    /// Install script URL (default https://get.docker.com; env `DOCKER_INSTALL_SCRIPT_URL`).
    #[arg(long, value_name = "URL")]
    pub install_script_url: Option<String>,
    /// Add this user to the docker group (e.g. `ubuntu`).
    #[arg(long, value_name = "USERNAME")]
    pub user: Option<String>,
    /// Docker daemon log driver (written to /etc/docker/daemon.json).
    #[arg(long, value_name = "DRIVER")]
    pub log_driver: Option<String>,
    /// Registry mirror URLs (repeatable, written to /etc/docker/daemon.json).
    #[arg(long = "registry-mirror", value_name = "URL")]
    pub registry_mirrors: Vec<String>,
    /// Re-install even if docker is already on PATH.
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long, short = 'y')]
    pub yes: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}

#[derive(Debug, clap::Args)]
#[command(about = "Install k3s lightweight Kubernetes distribution.")]
#[command(
    long_about = "Install k3s (server or agent) via official get.k3s.io installer. \
Idempotent: probes `command -v k3s` and skips download/install if found (use --force to override). \
Agent role requires --server-url / K3S_URL (https://<host>:<port>) and --token / K3S_TOKEN. \
Optional: pin version (--version / INSTALL_K3S_VERSION), extra k3s args (--install-exec / INSTALL_K3S_EXEC). \
Verification: runs `sudo k3s kubectl get nodes -o name` and requires at least one node. \
Supports --dry-run, --yes, --force, --output json."
)]
pub struct BootstrapK3sCommand {
    /// Install a k3s server (default) or join this host as an agent using `K3S_URL` / `K3S_TOKEN`.
    #[arg(long, value_enum, default_value_t = K3sRole::Server)]
    pub role: K3sRole,
    /// k3s API server URL for `--role agent` (e.g. `https://server:6443`; flag or env `K3S_URL`).
    #[arg(long)]
    pub server_url: Option<String>,
    /// Cluster secret / agent token for `--role agent` (flag or env `K3S_TOKEN`).
    #[arg(long)]
    pub token: Option<String>,
    /// Pin k3s version (`INSTALL_K3S_VERSION`; flag or env `INSTALL_K3S_VERSION`).
    #[arg(long)]
    pub version: Option<String>,
    /// Extra arguments for the k3s binary (`INSTALL_K3S_EXEC`).
    #[arg(long = "install-exec")]
    pub install_exec: Option<String>,
    /// Sets `INSTALL_K3S_SKIP_START=true` (binaries without starting the service).
    #[arg(long)]
    pub skip_start: bool,
    /// Sets `INSTALL_K3S_SKIP_ENABLE=true` (skip `systemctl enable`).
    #[arg(long)]
    pub skip_enable: bool,
    /// Re-run get.k3s.io install even when `k3s` is already on `PATH` (default: skip download/install if probe succeeds).
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub yes: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}

#[derive(Debug, clap::Args)]
#[command(about = "Install NetBird VPN overlay network client.")]
#[command(
    long_about = "Install NetBird via official https://pkgs.netbird.io/install.sh, optionally join a management service, and verify connectivity. \
Install method: --install-method binary (default, GitHub releases) or package (apt/dnf/yum, set DEBIAN_FRONTEND=noninteractive for quiet apt). \
Join: uses `netbird up` with optional --setup-key / NETBIRD_SETUP_KEY and --management-url / NETBIRD_MANAGEMENT_URL. \
Verify: runs `netbird status`, requires 'Management: Connected' and 'Signal: Connected' in output. \
Optional: --release / NETBIRD_RELEASE for version pin. Supports --dry-run, --yes, --output json."
)]
pub struct BootstrapNetbirdCommand {
    /// Skip NetBird UI packages (maps to SKIP_UI_APP for the official install script).
    #[arg(long)]
    pub skip_ui: bool,
    /// NetBird release version or `latest` (maps to NETBIRD_RELEASE).
    #[arg(long)]
    pub release: Option<String>,
    /// Setup key for headless join (prefer env NETBIRD_SETUP_KEY in CI).
    #[arg(long)]
    pub setup_key: Option<String>,
    /// Self-hosted management service URL (prefer env NETBIRD_MANAGEMENT_URL).
    #[arg(long)]
    pub management_url: Option<String>,
    /// Install via portable GitHub binaries (`USE_BIN_INSTALL=true`) or system packages (apt/dnf/yum). If omitted: env `HEIMDALL_NETBIRD_INSTALL_METHOD`, else `--yes`/`--dry-run` default to binary, else an interactive prompt.
    #[arg(long, value_enum)]
    pub install_method: Option<NetbirdInstallMethod>,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub yes: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}

#[derive(Debug, clap::Args)]
#[command(about = "Create or update an admin user with SSH public key authentication.")]
#[command(
    long_about = "Create or ensure user exists with SSH keys in authorized_keys (idempotent). \
Resolve username from --user or interactive prompt. Resolve SSH keys from --key / --key-file or interactive prompt. \
Validation: username format, SSH key algorithms (ssh-ed25519, ssh-rsa, ecdsa-sha2-nistp256). \
Operations: group creation, user creation, .ssh dir setup, authorized_keys management (atomic promotion), \
optional hardening (--disable-root-login, --disable-password-auth, requires --yes or confirmation). \
Idempotent: uses grep -qxF to skip duplicate keys. Stop-on-failure behavior. \
Supports --dry-run, --yes, --output json."
)]
pub struct BootstrapUserCommand {
    #[arg(long)]
    pub user: Option<String>,
    #[arg(long)]
    pub group: Option<String>,
    #[arg(long = "key-file")]
    pub key_files: Vec<String>,
    #[arg(long = "key")]
    pub keys: Vec<String>,
    /// User password for sudo authentication (prompted interactively if omitted)
    #[arg(long)]
    pub password: Option<String>,
    #[arg(long)]
    pub disable_root_login: bool,
    #[arg(long)]
    pub disable_password_auth: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub yes: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}

#[derive(Debug, Clone, clap::Args)]
#[command(
    about = "Install Infisical CLI and configure Universal Auth agent for secrets management."
)]
#[command(
    long_about = "Install Infisical CLI, authenticate, discover secrets folders, \
and deploy the Infisical Agent as a systemd service. \
Supports interactive folder discovery (via `infisical secrets folders list`) with fallback to manual specification. \
Securely writes Universal Auth credentials to {secrets_dir}/.infisical/ (mode 600). \
Generates agent.yaml with templates for root folder and each subfolder. \
Supports --dry-run, --skip-login, --yes, --output json. \
Environment vars: INFISICAL_ADDRESS, INFISICAL_PROJECT_SLUG, INFISICAL_ENV, INFISICAL_CLIENT_ID, INFISICAL_CLIENT_SECRET."
)]
pub struct BootstrapInfisicalCommand {
    /// Infisical API address (default: https://eu.infisical.com).
    #[arg(long)]
    pub address: Option<String>,

    /// Infisical project slug (required; prompted if TTY).
    #[arg(long)]
    pub project_slug: Option<String>,

    /// Infisical project ID (UUID; required; prompted if TTY).
    #[arg(long)]
    pub project_id: Option<String>,

    /// Secrets environment (default: prod).
    #[arg(long)]
    pub environment: Option<String>,

    /// Node name for folder discovery (default: hostname).
    #[arg(long)]
    pub node_name: Option<String>,

    /// Secrets subfolder names (repeatable; auto-discovered if omitted and logged in).
    #[arg(long = "folder")]
    pub folders: Vec<String>,

    /// Universal Auth Client ID (prompted if TTY).
    #[arg(long)]
    pub client_id: Option<String>,

    /// Universal Auth Client Secret (prompted if TTY).
    #[arg(long)]
    pub client_secret: Option<String>,

    /// Directory for secrets storage and credentials (default: /var/secrets).
    #[arg(long)]
    pub secrets_dir: Option<String>,

    /// Directory for Infisical Agent config (default: /etc/heimdall/infisical).
    #[arg(long)]
    pub config_dir: Option<String>,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub yes: bool,

    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}

#[derive(Debug, Clone, clap::Args)]
#[command(about = "Initialize and start a Komodo instance with Docker Compose.")]
#[command(
    long_about = "Deploy Komodo (core or periphery-only) via Docker Compose. \
Core mode (default): MongoDB + Komodo Core + Komodo Periphery full stack. \
Periphery mode: Periphery only, connects to remote Core instance. \
Generates compose.yaml and compose.env in /etc/heimdall/komodo/ (or --dir). \
Secrets auto-generated (admin password, database password, webhook/JWT secrets). \
Runs `docker compose up -d` by default (skip with --no-up). \
Supports --dry-run, --yes, --force, --output json."
)]
pub struct BootstrapKomodoCommand {
    /// Deployment mode: core (full stack) or periphery (remote core).
    #[arg(long, value_enum, default_value_t = KomodoMode::Core)]
    pub mode: KomodoMode,

    /// Output directory for compose.yaml + compose.env (default: /etc/heimdall/komodo).
    #[arg(long)]
    pub dir: Option<String>,

    /// Komodo image tag (default: 2).
    #[arg(long)]
    pub image_tag: Option<String>,

    /// Overwrite existing compose/env files.
    #[arg(long)]
    pub force: bool,

    /// Komodo Core host URL (e.g. https://komodo.example.com). Core mode only; prompted if omitted and TTY.
    #[arg(long)]
    pub host: Option<String>,

    /// Komodo title displayed in browser (default: Komodo). Core mode only.
    #[arg(long)]
    pub title: Option<String>,

    /// Host port for Komodo Core UI (default: 9120). Core mode only.
    #[arg(long)]
    pub port: Option<u16>,

    /// Admin username (default: admin). Prompted if omitted and TTY.
    #[arg(long)]
    pub admin_username: Option<String>,

    /// Admin password (auto-generated if omitted). Prompted if omitted and TTY.
    #[arg(long)]
    pub admin_password: Option<String>,

    /// Database username (default: admin). Core mode only.
    #[arg(long)]
    pub db_username: Option<String>,

    /// Database password (auto-generated if omitted). Prompted if omitted and TTY. Core mode only.
    #[arg(long)]
    pub db_password: Option<String>,

    /// Path for database backups (default: /etc/komodo/backups). Core mode only.
    #[arg(long)]
    pub backups_path: Option<String>,

    /// Server name for first server registration (default: Local). Core mode only.
    #[arg(long)]
    pub first_server_name: Option<String>,

    /// Core WebSocket address (e.g. ws://core:9120). Periphery mode only; required for periphery.
    #[arg(long)]
    pub core_address: Option<String>,

    /// Server name Periphery uses when connecting (default: same as first-server-name). Periphery mode only.
    #[arg(long)]
    pub connect_as: Option<String>,

    /// Path to Core's public key file (write to keys/core.pub). Periphery mode only; optional.
    #[arg(long)]
    pub core_public_key_file: Option<String>,

    /// Periphery root directory (default: /etc/komodo). Periphery mode only.
    #[arg(long)]
    pub periphery_root: Option<String>,

    /// Write compose files but do not run `docker compose up -d`.
    #[arg(long)]
    pub no_up: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub yes: bool,

    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}

#[derive(Debug, Subcommand)]
pub enum HardenAction {
    /// Harden firewall with toggle rules (SSH, established, HTTP, HTTPS) + custom ports.
    Firewall(HardenFirewallCommand),
    /// Harden SSH server (change port + optional toggles for root login, password auth).
    Ssh(HardenSshCommand),
}

#[derive(Debug, clap::Args)]
#[command(about = "Harden firewall with firewalld (Fedora).")]
#[command(
    long_about = "Configure firewalld with toggleable presets (SSH, established, HTTP, HTTPS) + custom port rules. \
Default: allow SSH (port from sshd_config) + established connections. \
SSH port is auto-detected from /etc/ssh/sshd_config or .heimdall/config.yaml override. \
Interactive mode: if no toggle flags given and stdin is a TTY, prompts for each preset. \
Sets default zone to drop (deny all inbound) unless already applied. \
Requires firewalld installed. Idempotent. Supports --dry-run, --yes, --output json."
)]
pub struct HardenFirewallCommand {
    /// Allow SSH access (default: true). Auto-detects port from sshd_config.
    #[arg(long, default_value = "true")]
    pub allow_ssh: bool,

    /// Allow established/related connections (default: true).
    #[arg(long, default_value = "true")]
    pub allow_established: bool,

    /// Allow HTTP (port 80).
    #[arg(long, default_value = "false")]
    pub allow_http: bool,

    /// Allow HTTPS (port 443).
    #[arg(long, default_value = "false")]
    pub allow_https: bool,

    /// Custom firewall rule. Format: port=<N>,protocol=<tcp|udp|both>. Repeatable.
    #[arg(long = "custom-rule")]
    pub custom_rules: Vec<String>,

    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompts.
    #[arg(long)]
    pub yes: bool,

    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}

#[derive(Debug, clap::Args)]
#[command(about = "Harden SSH server configuration.")]
#[command(
    long_about = "Change SSH port (with firewall safety checks) and optionally disable root login / password auth. \
Secure defaults: root login and password authentication are disabled unless you opt out (--allow-root-login / --allow-password-auth). \
Use interactive mode for a checkbox-style prompt over both options. \
Port change with full safety: backs up sshd_config, probes firewall, auto-opens port if needed (or errors without --yes). \
Runs sshd -t validation before reload. Idempotent: skips port change if already set. \
Requires explicit confirmation for risky operations unless --yes. \
Supports --dry-run, --output json."
)]
pub struct HardenSshCommand {
    /// New SSH port to set. Prompted if not given (requires TTY).
    #[arg(long)]
    pub port: Option<u16>,

    /// Allow SSH login as root (default: disable root login / PermitRootLogin no).
    #[arg(long)]
    pub allow_root_login: bool,

    /// Allow password authentication (default: disable password auth).
    #[arg(long)]
    pub allow_password_auth: bool,

    /// Disable root login (deprecated: same as default secure behavior; prefer omitting this flag).
    #[arg(long)]
    pub disable_root_login: bool,

    /// Disable password authentication (deprecated: same as default secure behavior).
    #[arg(long)]
    pub disable_password_auth: bool,

    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompts.
    #[arg(long)]
    pub yes: bool,

    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}

#[derive(Debug, clap::Args)]
pub struct HardenCommand {
    #[command(subcommand)]
    pub action: HardenAction,
}

#[derive(Debug, Subcommand)]
pub enum VerifyAction {
    /// Run environment diagnostics (cargo, git, current working directory accessibility).
    Doctor(VerifyDoctorCommand),
}

#[derive(Debug, clap::Args)]
pub struct VerifyCommand {
    #[command(subcommand)]
    pub action: VerifyAction,
}

#[derive(Debug, clap::Args)]
#[command(about = "Run environment diagnostics and report infrastructure health.")]
#[command(
    long_about = "Non-mutating checks: cargo availability, current working directory accessibility, git repository presence. \
Exit: 0 if no failures, 1 if any check fails. Supports --output json for structured results."
)]
pub struct VerifyDoctorCommand {
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}

#[derive(Debug, clap::Args)]
#[command(about = "Replace the running binary with a newer version from GitHub Releases.")]
#[command(
    long_about = "Fetch remote .sha256 from GitHub Releases (releases/{latest|tag}/heimdall-linux-amd64.sha256). \
Compare to SHA256 of the running binary; skip download if match (unless --force). Requires curl on PATH and write access to binary directory. \
Linux x86_64 only. Optional GITHUB_TOKEN for authentication (redacted in output). \
Supports --dry-run (resolve URLs + fetch checksum only), --yes (skip confirmation), --force (re-download), --tag (non-latest version), --output json."
)]
pub struct UpdateCommand {
    /// Print resolved URLs and digests; fetch remote `.sha256` only (no full binary download, no replace).
    #[arg(long)]
    pub dry_run: bool,
    /// Skip confirmation before replacing the running binary.
    #[arg(long)]
    pub yes: bool,
    /// Re-download and replace even when the remote digest matches the running binary (checksum verification still applies).
    #[arg(long)]
    pub force: bool,
    /// Generic package version string (default: `latest` rolling package from main).
    #[arg(long)]
    pub tag: Option<String>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}
