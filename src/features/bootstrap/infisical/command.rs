use crate::cli::BootstrapInfisicalCommand;
use crate::cli::GlobalOpts;
use crate::features::bootstrap::infisical::execute;
use crate::features::bootstrap::infisical::human;
use crate::features::bootstrap::infisical::input;
use crate::features::bootstrap::infisical::plan;
use crate::output::Style;
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
    crate::features::bootstrap::infisical::validate::validate_address(&config.address)?;

    // Build plan
    let operations = plan::build_plan(config)?;

    // Select I/O mode
    let io_mode = match config.output {
        crate::cli::OutputFormat::Human if !config.dry_run => IoMode::LiveTee,
        _ => IoMode::Buffered,
    };

    // Execute plan
    let runner = LocalRunner;
    let report = execute::execute_plan(&runner, operations, io_mode);

    // Format output
    let style = Style::for_human(global.color);
    match config.output {
        crate::cli::OutputFormat::Human => {
            print!("{}", human::format_report_human(&report, &style));
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
