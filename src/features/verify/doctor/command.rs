use anyhow::Result;

use crate::cli::{OutputFormat, VerifyDoctorCommand};
use crate::output::render_doctor_human;
use crate::runtime::ExitStatus;

use super::checks;

pub fn run(opts: VerifyDoctorCommand) -> Result<ExitStatus> {
    let report = checks::run();

    match opts.output {
        OutputFormat::Human => println!("{}", render_doctor_human(&report)),
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
