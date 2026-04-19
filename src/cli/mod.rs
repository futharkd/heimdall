use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
}

#[derive(Debug, Parser)]
#[command(name = "heimdall", version, about = "Modular infrastructure CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Bootstrap(BootstrapCommand),
    Harden(HardenCommand),
    Verify(VerifyCommand),
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
