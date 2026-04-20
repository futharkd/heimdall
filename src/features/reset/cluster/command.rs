use anyhow::Result;

use crate::cli::{GlobalOpts, OutputFormat, ResetClusterCommand};
use crate::output::Style;
use crate::runner::{IoMode, LocalRunner};
use crate::runtime::ExitStatus;

use super::execute::execute_plan;
use super::human::format_report_human;
use super::input::resolve_inputs;
use super::plan::build_plan;

pub fn run(opts: ResetClusterCommand, global: &GlobalOpts) -> Result<ExitStatus> {
    let resolved = resolve_inputs(opts)?;
    let runner = LocalRunner;
    let plan = build_plan(&resolved.config)?;
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

    use crate::cli::{Cli, Command, OutputFormat, ResetAction};

    #[test]
    fn cli_parses_reset_cluster_flags() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "reset",
            "cluster",
            "--dry-run",
            "--yes",
            "--confirm",
            "reset-cluster",
            "--output",
            "json",
        ])
        .expect("cli parses");

        let Command::Reset(reset) = parsed.command else {
            panic!("expected reset");
        };
        let c = match reset.action {
            ResetAction::Cluster(c) => c,
        };
        assert!(c.dry_run);
        assert!(c.yes);
        assert_eq!(c.confirm.as_deref(), Some("reset-cluster"));
        assert!(matches!(c.output, OutputFormat::Json));
    }
}
