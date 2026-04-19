pub mod verify;

use anyhow::Result;

use crate::cli::{BootstrapAction, Command, HardenAction, VerifyAction};
use crate::runtime::ExitStatus;

pub fn dispatch(cli: crate::cli::Cli) -> Result<ExitStatus> {
    match cli.command {
        Command::Verify(cmd) => match cmd.action {
            VerifyAction::Doctor(opts) => verify::doctor(opts),
        },
        Command::Bootstrap(cmd) => match cmd.action {
            BootstrapAction::Flux => {
                println!("bootstrap flux is scaffolded but not implemented yet");
                Ok(ExitStatus::Warning)
            }
            BootstrapAction::User => {
                println!("bootstrap user is scaffolded but not implemented yet");
                Ok(ExitStatus::Warning)
            }
        },
        Command::Harden(cmd) => match cmd.action {
            HardenAction::Ssh => {
                println!("harden ssh is scaffolded but not implemented yet");
                Ok(ExitStatus::Warning)
            }
        },
    }
}
