use anyhow::Result;

use crate::cli::{BootstrapUserCommand, GlobalOpts, OutputFormat};
use crate::output::{Style, execution_footer_line};
use crate::runner::{IoMode, LocalRunner};
use crate::runtime::ExitStatus;

use super::execute::execute_plan;
use super::human::format_report_human;
use super::input::resolve_inputs;
use super::plan::build_plan;

pub fn run(opts: BootstrapUserCommand, global: &GlobalOpts) -> Result<ExitStatus> {
    let resolved = resolve_inputs(opts)?;
    let plan = build_plan(&resolved.config)?;
    let runner = LocalRunner;
    let live_execution = matches!(
        (resolved.output, resolved.config.dry_run),
        (OutputFormat::Human, false)
    );
    let io_mode = if live_execution {
        IoMode::LiveTee
    } else {
        IoMode::Buffered
    };
    let report = execute_plan(&runner, &resolved.config, &plan, io_mode);

    let style = match resolved.output {
        OutputFormat::Human => Style::for_human(global.color),
        OutputFormat::Json => Style::plain(),
    };

    match resolved.output {
        OutputFormat::Human if live_execution => {
            println!("{}", execution_footer_line(&report.operations))
        }
        OutputFormat::Human => println!("{}", format_report_human(&report, &style)),
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
    }

    Ok(if report.has_failures() {
        ExitStatus::Failure
    } else {
        ExitStatus::Success
    })
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::cli::{BootstrapAction, Cli, Command, OutputFormat};

    #[test]
    fn cli_parses_bootstrap_user_flags() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "bootstrap",
            "user",
            "--user",
            "admin",
            "--key",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIabc123 admin@host",
            "--output",
            "json",
        ])
        .expect("cli should parse");

        let Command::Bootstrap(bootstrap) = parsed.command else {
            panic!("expected bootstrap command");
        };

        let BootstrapAction::User(user) = bootstrap.action else {
            panic!("expected user action");
        };

        assert_eq!(user.user.as_deref(), Some("admin"));
        assert!(matches!(user.output, OutputFormat::Json));
    }
}
