use anyhow::Result;

use crate::cli::{BootstrapK3sCommand, GlobalOpts, OutputFormat};
use crate::output::{Style, execution_footer_line};
use crate::runner::{IoMode, LocalRunner};
use crate::runtime::ExitStatus;

use super::execute::execute_plan;
use super::human::format_report_human;
use super::input::{probe_k3s_on_path, resolve_inputs};
use super::plan::build_plan;

pub fn run(opts: BootstrapK3sCommand, global: &GlobalOpts) -> Result<ExitStatus> {
    let mut resolved = resolve_inputs(opts)?;
    let runner = LocalRunner;
    if !resolved.config.force && probe_k3s_on_path(&runner) {
        resolved.config.skip_install = true;
        eprintln!(
            "note: k3s found on PATH; skipping get.k3s.io download and install (use --force to re-run installer)"
        );
    }
    let plan = build_plan(&resolved.config)?;
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

    use crate::cli::{BootstrapAction, Cli, Command, K3sRole, OutputFormat};

    #[test]
    fn cli_parses_bootstrap_k3s_flags() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "bootstrap",
            "k3s",
            "--role",
            "agent",
            "--server-url",
            "https://cp:6443",
            "--token",
            "tok",
            "--version",
            "v1.30.1+k3s1",
            "--install-exec=--node-name=testnode",
            "--dry-run",
            "--output",
            "json",
        ])
        .expect("cli parses");

        let Command::Bootstrap(bootstrap) = parsed.command else {
            panic!("expected bootstrap");
        };
        let BootstrapAction::K3s(k3s) = bootstrap.action else {
            panic!("expected k3s");
        };
        assert_eq!(k3s.role, K3sRole::Agent);
        assert_eq!(k3s.server_url.as_deref(), Some("https://cp:6443"));
        assert_eq!(k3s.token.as_deref(), Some("tok"));
        assert_eq!(k3s.version.as_deref(), Some("v1.30.1+k3s1"));
        assert_eq!(k3s.install_exec.as_deref(), Some("--node-name=testnode"));
        assert!(k3s.dry_run);
        assert!(!k3s.force);
        assert!(matches!(k3s.output, OutputFormat::Json));
    }

    #[test]
    fn cli_parses_bootstrap_k3s_force() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "bootstrap",
            "k3s",
            "--force",
            "--dry-run",
            "--yes",
        ])
        .expect("cli parses");
        let Command::Bootstrap(bootstrap) = parsed.command else {
            panic!("expected bootstrap");
        };
        let BootstrapAction::K3s(k3s) = bootstrap.action else {
            panic!("expected k3s");
        };
        assert!(k3s.force);
    }
}
