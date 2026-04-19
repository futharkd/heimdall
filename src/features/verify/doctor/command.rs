use anyhow::Result;

use crate::cli::{GlobalOpts, OutputFormat, VerifyDoctorCommand};
use crate::output::Style;
use crate::runner::IoMode;
use crate::runtime::ExitStatus;

use super::checks;
use super::human::format_report_human;

pub fn run(opts: VerifyDoctorCommand, global: &GlobalOpts) -> Result<ExitStatus> {
    let io_mode = match opts.output {
        OutputFormat::Human => IoMode::LiveTee,
        OutputFormat::Json => IoMode::Buffered,
    };
    let report = checks::run(io_mode);

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

    use crate::cli::{Cli, Command, OutputFormat, VerifyAction};

    #[test]
    fn cli_parses_verify_doctor_json() {
        let parsed = Cli::try_parse_from(["heimdall", "verify", "doctor", "--output", "json"])
            .expect("cli should parse");

        let Command::Verify(verify) = parsed.command else {
            panic!("expected verify command");
        };

        let VerifyAction::Doctor(doctor) = verify.action;
        assert!(matches!(doctor.output, OutputFormat::Json));
    }
}
