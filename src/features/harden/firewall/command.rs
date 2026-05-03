use super::execute::execute_plan;
use super::human::format_report_human;
use super::input::resolve_inputs;
use super::plan::build_plan;
use crate::cli::HardenFirewallCommand;
use crate::config;
use crate::output::Style;
use crate::runner::{IoMode, LocalRunner};
use crate::runtime::ExitStatus;
use anyhow::Result;
use serde_json::json;

pub fn run(opts: HardenFirewallCommand, global: &crate::cli::GlobalOpts) -> Result<ExitStatus> {
    let resolved = resolve_inputs(opts)?;
    let runner = LocalRunner;
    let style = Style::for_human(global.color);

    // Build the plan
    let plan = build_plan(&resolved.config)?;

    // Determine IO mode
    let io_mode =
        if matches!(resolved.output, crate::cli::OutputFormat::Json) || resolved.config.dry_run {
            IoMode::Buffered
        } else {
            IoMode::LiveTee
        };

    // Execute plan
    let report = execute_plan(&runner, &resolved.config, &plan, io_mode);

    // Format and output report
    match resolved.output {
        crate::cli::OutputFormat::Human => {
            let formatted = format_report_human(&report, &style);
            println!("{}", formatted);
        }
        crate::cli::OutputFormat::Json => {
            // Convert report to JSON-serializable format
            let json_report = json!({
                "operations": report.operations.iter().map(|op| {
                    json!({
                        "id": op.id,
                        "description": op.description,
                        "status": format!("{:?}", op.status).to_lowercase(),
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

        // Ensure harden config exists
        if config.harden.is_none() {
            config.harden = Some(crate::config::HardenConfig::default());
        }

        let harden = config.harden.as_mut().unwrap();

        // Ensure firewall config exists
        if harden.firewall.is_none() {
            harden.firewall = Some(crate::config::FirewallHardenState::default());
        }

        let firewall = harden.firewall.as_mut().unwrap();
        firewall.applied = true;
        firewall.presets = vec![];
        if resolved.config.allow_ssh {
            firewall.presets.push("ssh".to_string());
        }
        if resolved.config.allow_established {
            firewall.presets.push("established".to_string());
        }
        if resolved.config.allow_http {
            firewall.presets.push("http".to_string());
        }
        if resolved.config.allow_https {
            firewall.presets.push("https".to_string());
        }
        firewall.custom_rules = resolved
            .config
            .custom_rules
            .iter()
            .map(|r| crate::config::CustomFirewallRule {
                port: r.port,
                protocol: r.protocol.clone(),
            })
            .collect();

        config::save(&config, &config_path)?;
    }

    if report.has_failures() {
        Ok(ExitStatus::Failure)
    } else {
        Ok(ExitStatus::Success)
    }
}
