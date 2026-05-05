use crate::cli::BootstrapInfisicalCommand;
use crate::cli::GlobalOpts;
use crate::core::elevation::PrivilegeContext;
use crate::features::bootstrap::infisical::execute;
use crate::features::bootstrap::infisical::human;
use crate::features::bootstrap::infisical::input;
use crate::features::bootstrap::infisical::plan;
use crate::output::{Style, execution_footer_line};
use crate::runner::IoMode;
use crate::runner::LocalRunner;
use crate::runtime::ExitStatus;
use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

pub fn run(opts: BootstrapInfisicalCommand, global: &GlobalOpts) -> Result<ExitStatus> {
    // Resolve inputs
    let mut resolved = input::resolve_inputs(opts)?;
    let config = &mut resolved.config;

    // Check if infisical is already installed (idempotency probe)
    if Command::new("sh")
        .args(["-c", "command -v infisical"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        config.skip_install = true;
    }

    // Validate inputs
    crate::core::validation::ensure_valid(
        &crate::features::bootstrap::infisical::validate::AddressValidator {
            address: &config.address,
        },
    )?;

    // Build plan
    let artifacts = plan::resolve_plan_artifacts(config)?;
    let operations = plan::build_plan(config, &artifacts)?;

    // Select I/O mode
    let live_execution =
        matches!(config.output, crate::cli::OutputFormat::Human) && !config.dry_run;
    let io_mode = if live_execution {
        IoMode::LiveTee
    } else {
        IoMode::Buffered
    };

    // Execute plan
    let runner = LocalRunner;
    let report = execute::execute_plan(
        &runner,
        PrivilegeContext::ELEVATED_OPS,
        operations,
        io_mode,
        Some(config.environment.clone()),
    );

    // Format output
    let style = Style::for_human(global.color);
    match config.output {
        crate::cli::OutputFormat::Human => {
            if live_execution {
                println!("{}", execution_footer_line(&report.operations));
            } else {
                print!("{}", human::format_report_human(&report, &style));
            }
        }
        crate::cli::OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    if !report.has_failures() && !config.dry_run {
        let (mut hcfg, path) = crate::config::load().unwrap_or_else(|_| {
            (
                crate::config::HeimdallConfig::default(),
                PathBuf::from("/etc/heimdall/config.yaml"),
            )
        });
        let bootstrap = hcfg
            .bootstrap
            .get_or_insert_with(crate::config::BootstrapConfig::default);
        bootstrap.infisical = Some(crate::config::InfisicalState {
            address: Some(config.address.clone()),
            project_id: Some(config.project_id.clone()),
            project_slug: Some(config.project_slug.clone()),
            environment: Some(config.environment.clone()),
            node_name: Some(config.node_name.clone()),
            secrets_dir: Some(config.secrets_dir.clone()),
            config_dir: Some(config.config_dir.clone()),
            folders: artifacts.folders.clone(),
        });
        if let Err(e) = crate::config::save(&hcfg, &path) {
            eprintln!("warning: failed to persist heimdall config: {e}");
        }
    }

    if report.has_failures() {
        Ok(ExitStatus::Failure)
    } else {
        Ok(ExitStatus::Success)
    }
}
