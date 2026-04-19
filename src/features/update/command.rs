use anyhow::Result;

use crate::cli::{OutputFormat, UpdateCommand};
use crate::output::render_update_human;
use crate::runner::LocalRunner;
use crate::runtime::ExitStatus;

use super::execute::execute_update;
use super::input::resolve_inputs;

pub fn run(opts: UpdateCommand) -> Result<ExitStatus> {
    let config = resolve_inputs(opts)?;
    let output = config.output;
    let runner = LocalRunner;
    let report = execute_update(&runner, &config);

    match output {
        OutputFormat::Human => println!("{}", render_update_human(&report)),
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
    fn cli_parses_update_flags() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "update",
            "--dry-run",
            "--force",
            "--yes",
            "--tag",
            "v0.2.0",
            "--output",
            "json",
        ])
        .expect("cli should parse");

        let Command::Update(update) = parsed.command else {
            panic!("expected update command");
        };

        assert!(update.dry_run);
        assert!(update.force);
        assert!(update.yes);
        assert_eq!(update.tag.as_deref(), Some("v0.2.0"));
        assert!(matches!(update.output, OutputFormat::Json));
    }
}
