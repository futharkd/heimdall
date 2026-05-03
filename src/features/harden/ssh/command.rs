use super::execute::execute_plan;
use super::human::format_report_human;
use super::input::resolve_inputs;
use super::plan::build_plan;
use crate::cli::HardenSshCommand;
use crate::config;
use crate::output::Style;
use crate::runner::{IoMode, LocalRunner};
use crate::runtime::ExitStatus;
use anyhow::Result;
use serde_json::json;

pub fn run(opts: HardenSshCommand, global: &crate::cli::GlobalOpts) -> Result<ExitStatus> {
    let resolved = resolve_inputs(opts)?;
    let runner = LocalRunner;
    let style = Style::for_human(global.color);

    let plan = build_plan(&resolved.config)?;

    let io_mode =
        if matches!(resolved.output, crate::cli::OutputFormat::Json) || resolved.config.dry_run {
            IoMode::Buffered
        } else {
            IoMode::LiveTee
        };

    let report = execute_plan(&runner, &resolved.config, &plan, io_mode);

    match resolved.output {
        crate::cli::OutputFormat::Human => {
            let formatted = format_report_human(&report, &style);
            println!("{}", formatted);
        }
        crate::cli::OutputFormat::Json => {
            let json_report = json!({
                "operations": report.operations.iter().map(|op| {
                    json!({
                        "id": op.id,
                        "description": op.description,
                        "status": op.status,
                        "detail": op.detail,
                    })
                }).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&json_report)?);
        }
    }

    // Save config if successful
    if !report.has_failures() && !resolved.config.dry_run {
        let (mut config, config_path) = config::load()?;

        if config.harden.is_none() {
            config.harden = Some(crate::config::HardenConfig::default());
        }

        let harden = config.harden.as_mut().unwrap();

        if harden.ssh.is_none() {
            harden.ssh = Some(crate::config::SshHardenState::default());
        }

        let ssh = harden.ssh.as_mut().unwrap();
        if let Some(port) = resolved.config.new_port {
            ssh.port = Some(port);
        }
        ssh.root_login_disabled = resolved.config.disable_root_login;
        ssh.password_auth_disabled = resolved.config.disable_password_auth;

        config::save(&config, &config_path)?;
    }

    if report.has_failures() {
        Ok(ExitStatus::Failure)
    } else {
        Ok(ExitStatus::Success)
    }
}
