use anyhow::Result;

use crate::cli::{BootstrapAction, Command, HardenAction, VerifyAction};
use crate::features::{bootstrap, update, verify};
use crate::runtime::ExitStatus;

pub fn dispatch(cli: crate::cli::Cli) -> Result<ExitStatus> {
    match cli.command {
        Command::Verify(cmd) => match cmd.action {
            VerifyAction::Doctor(opts) => verify::doctor::command::run(opts, &cli.global),
        },
        Command::Bootstrap(cmd) => match cmd.action {
            BootstrapAction::Flux(opts) => bootstrap::flux::command::run(opts, &cli.global),
            BootstrapAction::K3s(opts) => bootstrap::k3s::command::run(opts, &cli.global),
            BootstrapAction::Netbird(opts) => bootstrap::netbird::command::run(opts, &cli.global),
            BootstrapAction::User(opts) => bootstrap::user::command::run(opts, &cli.global),
        },
        Command::Harden(cmd) => match cmd.action {
            HardenAction::Ssh => {
                println!("harden ssh is scaffolded but not implemented yet");
                Ok(ExitStatus::Warning)
            }
        },
        Command::Update(opts) => update::command::run(opts, &cli.global),
    }
}
