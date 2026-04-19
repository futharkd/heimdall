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
    User,
}

#[derive(Debug, clap::Args)]
pub struct BootstrapCommand {
    #[command(subcommand)]
    pub action: BootstrapAction,
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
