use anyhow::Result;

use crate::cli::{BootstrapAction, Command, HardenAction, VerifyAction};
use crate::features::{bootstrap, verify};
use crate::runtime::ExitStatus;

pub fn dispatch(cli: crate::cli::Cli) -> Result<ExitStatus> {
    match cli.command {
        Command::Verify(cmd) => match cmd.action {
            VerifyAction::Doctor(opts) => verify::doctor::command::run(opts),
        },
        Command::Bootstrap(cmd) => match cmd.action {
            BootstrapAction::Flux => {
                println!("bootstrap flux is scaffolded but not implemented yet");
                Ok(ExitStatus::Warning)
            }
            BootstrapAction::User(opts) => bootstrap::user::command::run(opts),
        },
        Command::Harden(cmd) => match cmd.action {
            HardenAction::Ssh => {
                println!("harden ssh is scaffolded but not implemented yet");
                Ok(ExitStatus::Warning)
            }
        },
    }
}
