use anyhow::Result;

use crate::cli::{BootstrapDockerCommand, GlobalOpts, OutputFormat};
use crate::output::Style;
use crate::runner::{IoMode, LocalRunner};
use crate::runtime::ExitStatus;

use super::execute::execute_plan;
use super::human::format_report_human;
use super::input::{probe_docker_on_path, resolve_inputs};
use super::plan::build_plan;

pub fn run(opts: BootstrapDockerCommand, global: &GlobalOpts) -> Result<ExitStatus> {
    let mut resolved = resolve_inputs(opts)?;
    let runner = LocalRunner;

    if !resolved.config.force && probe_docker_on_path(&runner) {
        resolved.config.skip_install = true;
        eprintln!(
            "note: docker found on PATH; skipping get.docker.com download and install (use --force to re-run installer)"
        );
    }

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

    use crate::cli::{BootstrapAction, Cli, Command, OutputFormat};

    #[test]
    fn cli_parses_bootstrap_docker_flags() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "bootstrap",
            "docker",
            "--user",
            "ubuntu",
            "--log-driver",
            "json-file",
            "--registry-mirror",
            "https://mirror.example.com",
            "--dry-run",
            "--output",
            "json",
        ])
        .expect("cli parses");

        let Command::Bootstrap(bootstrap) = parsed.command else {
            panic!("expected bootstrap");
        };
        let BootstrapAction::Docker(docker) = bootstrap.action else {
            panic!("expected docker");
        };
        assert_eq!(docker.user.as_deref(), Some("ubuntu"));
        assert_eq!(docker.log_driver.as_deref(), Some("json-file"));
        assert_eq!(docker.registry_mirrors, vec!["https://mirror.example.com"]);
        assert!(docker.dry_run);
        assert!(!docker.force);
        assert!(matches!(docker.output, OutputFormat::Json));
    }

    #[test]
    fn cli_parses_bootstrap_docker_force() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "bootstrap",
            "docker",
            "--force",
            "--dry-run",
            "--yes",
        ])
        .expect("cli parses");

        let Command::Bootstrap(bootstrap) = parsed.command else {
            panic!("expected bootstrap");
        };
        let BootstrapAction::Docker(docker) = bootstrap.action else {
            panic!("expected docker");
        };
        assert!(docker.force);
        assert!(docker.dry_run);
        assert!(docker.yes);
    }

    #[test]
    fn cli_parses_bootstrap_docker_custom_url() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "bootstrap",
            "docker",
            "--install-script-url",
            "https://custom.example.com/install.sh",
            "--dry-run",
        ])
        .expect("cli parses");

        let Command::Bootstrap(bootstrap) = parsed.command else {
            panic!("expected bootstrap");
        };
        let BootstrapAction::Docker(docker) = bootstrap.action else {
            panic!("expected docker");
        };
        assert_eq!(
            docker.install_script_url.as_deref(),
            Some("https://custom.example.com/install.sh")
        );
    }
}
