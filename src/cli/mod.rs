use clap::{Parser, Subcommand, ValueEnum};

use crate::output::ColorArg;

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
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
    Bootstrap(BootstrapCommand),
    Harden(HardenCommand),
    Verify(VerifyCommand),
    Update(UpdateCommand),
}

#[derive(Debug, Subcommand)]
pub enum BootstrapAction {
    Flux(BootstrapFluxCommand),
    K3s(BootstrapK3sCommand),
    Netbird(BootstrapNetbirdCommand),
    User(BootstrapUserCommand),
}

#[derive(Debug, clap::Args)]
pub struct BootstrapCommand {
    #[command(subcommand)]
    pub action: BootstrapAction,
}

#[derive(Debug, clap::Args)]
pub struct BootstrapFluxCommand {
    /// SSH Git URL (e.g. `ssh://git@gitlab.com/group/repo.git`). Omit to use `FLUX_GIT_URL` or an interactive prompt when stdin is a TTY.
    #[arg(long)]
    pub url: Option<String>,
    /// Git branch (default `main`; env `FLUX_GIT_BRANCH`).
    #[arg(long)]
    pub branch: Option<String>,
    /// Path inside the repo for Flux manifests (e.g. `clusters/prod`).
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
pub struct BootstrapUserCommand {
    #[arg(long)]
    pub user: Option<String>,
    #[arg(long)]
    pub group: Option<String>,
    #[arg(long = "key-file")]
    pub key_files: Vec<String>,
    #[arg(long = "key")]
    pub keys: Vec<String>,
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

#[derive(Debug, Subcommand)]
pub enum HardenAction {
    Ssh,
}

#[derive(Debug, clap::Args)]
pub struct HardenCommand {
    #[command(subcommand)]
    pub action: HardenAction,
}

#[derive(Debug, Subcommand)]
pub enum VerifyAction {
    Doctor(VerifyDoctorCommand),
}

#[derive(Debug, clap::Args)]
pub struct VerifyCommand {
    #[command(subcommand)]
    pub action: VerifyAction,
}

#[derive(Debug, clap::Args)]
pub struct VerifyDoctorCommand {
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,
}

#[derive(Debug, clap::Args)]
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
