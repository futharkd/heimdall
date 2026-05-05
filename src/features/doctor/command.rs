use anyhow::Result;

use crate::cli::{DoctorCommand, GlobalOpts, OutputFormat};
use crate::core::doctor::DoctorContext;
use crate::output::Style;
use crate::runner::{IoMode, LocalRunner};
use crate::runtime::ExitStatus;

use super::human::format_report_human;
use super::registry;

pub fn run(opts: DoctorCommand, global: &GlobalOpts) -> Result<ExitStatus> {
    let io_mode = IoMode::Buffered;
    let runner = LocalRunner;
    let ctx = DoctorContext {
        runner: &runner,
        io_mode,
    };
    let report = registry::build_report(&ctx);

    let style = match opts.output {
        OutputFormat::Human => Style::for_human(global.color),
        OutputFormat::Json => Style::plain(),
    };

    match opts.output {
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

    use crate::cli::{Cli, Command, OutputFormat};

    #[test]
    fn cli_parses_doctor_json() {
        let parsed = Cli::try_parse_from(["heimdall", "doctor", "--output", "json"]).expect("cli");
        let Command::Doctor(doctor) = parsed.command else {
            panic!("expected doctor command");
        };
        assert!(matches!(doctor.output, OutputFormat::Json));
    }
}
