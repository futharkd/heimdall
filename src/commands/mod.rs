use anyhow::Result;

use crate::cli::{BootstrapAction, Command, HardenAction, ResetAction, ServiceTarget};
use crate::features::{bootstrap, doctor, harden, reset, service, update};
use crate::runtime::ExitStatus;

pub fn dispatch(cli: crate::cli::Cli) -> Result<ExitStatus> {
    match cli.command {
        Command::Doctor(opts) => doctor::command::run(opts, &cli.global),
        Command::Bootstrap(cmd) => match cmd.action {
            BootstrapAction::Docker(opts) => bootstrap::docker::command::run(opts, &cli.global),
            BootstrapAction::Flux(opts) => bootstrap::flux::command::run(opts, &cli.global),
            BootstrapAction::Infisical(opts) => {
                bootstrap::infisical::command::run(opts, &cli.global)
            }
            BootstrapAction::K3s(opts) => bootstrap::k3s::command::run(opts, &cli.global),
            BootstrapAction::Komodo(opts) => bootstrap::komodo::command::run(opts, &cli.global),
            BootstrapAction::Netbird(opts) => bootstrap::netbird::command::run(opts, &cli.global),
            BootstrapAction::User(opts) => bootstrap::user::command::run(opts, &cli.global),
        },
        Command::Harden(cmd) => match cmd.action {
            HardenAction::Firewall(opts) => harden::firewall::command::run(opts, &cli.global),
            HardenAction::Ssh(opts) => harden::ssh::command::run(opts, &cli.global),
        },
        Command::Reset(cmd) => match cmd.action {
            ResetAction::Cluster(opts) => reset::cluster::command::run(opts, &cli.global),
        },
        Command::Update(opts) => update::command::run(opts, &cli.global),
        Command::Service(cmd) => match cmd.target {
            ServiceTarget::Komodo(opts) => service::komodo::run(opts, &cli.global),
            ServiceTarget::Infisical(opts) => service::infisical::run(opts, &cli.global),
            ServiceTarget::Netbird(opts) => service::netbird::run(opts, &cli.global),
        },
    }
}
