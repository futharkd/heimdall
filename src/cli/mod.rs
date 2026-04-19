use clap::{Parser, Subcommand, ValueEnum};

use crate::output::ColorArg;

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
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
    Flux,
    Netbird(BootstrapNetbirdCommand),
    User(BootstrapUserCommand),
}

#[derive(Debug, clap::Args)]
pub struct BootstrapCommand {
    #[command(subcommand)]
    pub action: BootstrapAction,
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
