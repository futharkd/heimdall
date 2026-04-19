use anyhow::Result;

use crate::cli::{BootstrapNetbirdCommand, GlobalOpts, OutputFormat};
use crate::output::Style;
use crate::runner::{IoMode, LocalRunner};
use crate::runtime::ExitStatus;

use super::execute::execute_plan;
use super::human::format_report_human;
use super::input::resolve_inputs;
use super::plan::build_plan;

pub fn run(opts: BootstrapNetbirdCommand, global: &GlobalOpts) -> Result<ExitStatus> {
    let resolved = resolve_inputs(opts)?;
    let plan = build_plan(&resolved.config)?;
    let runner = LocalRunner;
    let io_mode = match (resolved.output, resolved.config.dry_run) {
        (OutputFormat::Human, false) => IoMode::LiveTee,
        _ => IoMode::Buffered,
    };
    let report = execute_plan(&runner, &resolved.config, &plan, io_mode);

    let style = match resolved.output {
        OutputFormat::Human => Style::for_human(global.color),
        OutputFormat::Json => Style::plain(),
    };

    match resolved.output {
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

    use crate::cli::{BootstrapAction, Cli, Command, NetbirdInstallMethod, OutputFormat};

    #[test]
    fn cli_parses_bootstrap_netbird_flags() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "bootstrap",
            "netbird",
            "--skip-ui",
            "--release",
            "latest",
            "--install-method",
            "package",
            "--dry-run",
            "--output",
            "json",
        ])
        .expect("cli parses");

        let Command::Bootstrap(bootstrap) = parsed.command else {
            panic!("expected bootstrap");
        };
        let BootstrapAction::Netbird(nb) = bootstrap.action else {
            panic!("expected netbird");
        };
        assert!(nb.skip_ui);
        assert_eq!(nb.release.as_deref(), Some("latest"));
        assert!(matches!(
            nb.install_method,
            Some(NetbirdInstallMethod::Package)
        ));
        assert!(nb.dry_run);
        assert!(matches!(nb.output, OutputFormat::Json));
    }
}
