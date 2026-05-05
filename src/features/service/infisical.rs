use anyhow::Result;
use std::path::PathBuf;

use crate::cli::{InfisicalServiceAction, ServiceInfisicalCommand};
use crate::core::operation::{OperationKind, PlannedOperation};
use crate::features::service::{
    ServiceActionKind, ServiceActionPlan, ServiceBackend, ServiceKind, execute_and_print,
    plan_service_actions,
};
use crate::runner::IoMode;
use crate::runner::LocalRunner;
use crate::runner::read::read_file_with_escalation;
use crate::runtime::ExitStatus;

pub fn run(opts: ServiceInfisicalCommand, global: &crate::cli::GlobalOpts) -> Result<ExitStatus> {
    match opts.action {
        InfisicalServiceAction::Sync => run_sync(opts, global),
        _ => run_standard(opts, global),
    }
}

fn run_standard(
    opts: ServiceInfisicalCommand,
    global: &crate::cli::GlobalOpts,
) -> Result<ExitStatus> {
    let action: ServiceActionKind = opts.action.into();

    let plan = ServiceActionPlan {
        kind: ServiceKind::Infisical,
        action,
        backend: ServiceBackend::Systemd {
            unit_name: opts.unit_name.clone(),
        },
    };

    let mut ops: Vec<PlannedOperation> = plan_service_actions(plan);
    if matches!(action, ServiceActionKind::Status) {
        ops.push(PlannedOperation {
            id: "service_infisical_cli_status",
            description: "Check infisical CLI on PATH".to_string(),
            kind: OperationKind::Shell {
                command: "command".to_string(),
                args: vec!["-v".to_string(), "infisical".to_string()],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: true,
            verify: None,
        });
    }

    let show_stdout = matches!(action, ServiceActionKind::Status);
    execute_and_print(
        "infisical",
        ops,
        opts.output,
        opts.dry_run,
        global,
        show_stdout,
    )
}

fn run_sync(opts: ServiceInfisicalCommand, global: &crate::cli::GlobalOpts) -> Result<ExitStatus> {
    let (hcfg, cfg_path) = crate::config::load()?;
    let saved_state = hcfg
        .bootstrap
        .as_ref()
        .and_then(|b| b.infisical.as_ref())
        .ok_or_else(|| {
            anyhow::anyhow!("Infisical not bootstrapped; run `bootstrap infisical` first")
        })?;

    let secrets_dir = saved_state
        .secrets_dir
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("secrets_dir not found in saved state"))?;
    let config_dir = saved_state
        .config_dir
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("config_dir not found in saved state"))?;
    let node_name = saved_state
        .node_name
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("node_name not found in saved state"))?;
    let project_id = saved_state
        .project_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("project_id not found in saved state"))?;
    let address = saved_state
        .address
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("address not found in saved state"))?;
    let project_slug = saved_state
        .project_slug
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("project_slug not found in saved state"))?;
    let environment = saved_state
        .environment
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("environment not found in saved state"))?;

    let creds_dir = format!("{}/.infisical", secrets_dir);
    let client_id_path = PathBuf::from(format!("{}/client-id", creds_dir));
    let client_secret_path = PathBuf::from(format!("{}/client-secret", creds_dir));

    let runner = LocalRunner;
    let client_id = read_file_with_escalation(&runner, &client_id_path, IoMode::Buffered)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", client_id_path.display(), e))?;
    let client_secret = read_file_with_escalation(&runner, &client_secret_path, IoMode::Buffered)
        .map_err(|e| {
        anyhow::anyhow!("failed to read {}: {}", client_secret_path.display(), e)
    })?;

    let token = crate::features::bootstrap::infisical::plan::universal_auth_token(
        client_id.trim(),
        client_secret.trim(),
        address,
    )?;

    let discovered_folders =
        crate::features::bootstrap::infisical::plan::discover_folders_recursive(
            address, project_id, &token, node_name,
        );

    if discovered_folders == saved_state.folders {
        println!("No folder changes detected.");
        return Ok(ExitStatus::Success);
    }

    let config = crate::features::bootstrap::infisical::input::BootstrapInfisicalConfig {
        address: address.clone(),
        project_slug: project_slug.clone(),
        project_id: project_id.clone(),
        environment: environment.clone(),
        node_name: node_name.clone(),
        folders: discovered_folders.clone(),
        client_id: client_id.trim().to_string(),
        client_secret: client_secret.trim().to_string(),
        secrets_dir: secrets_dir.clone(),
        config_dir: config_dir.clone(),
        skip_install: true,
        dry_run: opts.dry_run,
        output: opts.output,
    };

    let mut ops = vec![];

    for folder in &discovered_folders {
        if !saved_state.folders.contains(folder) {
            ops.push(PlannedOperation {
                id: "create_new_secret_subdir",
                description: format!("Create secrets output subdirectory for {}", folder),
                kind: OperationKind::Shell {
                    command: "mkdir".to_string(),
                    args: vec!["-p".to_string(), format!("{}/{}", secrets_dir, folder)],
                    env: vec![],
                    stdin_input: None,
                },
                requires_confirmation: false,
                failure_is_warning: false,
                verify: None,
            });
        }
    }

    let agent_yaml_content = crate::features::bootstrap::infisical::generate::agent_yaml(&config);
    let config_path = PathBuf::from(config_dir).join("agent.yaml");

    ops.push(PlannedOperation {
        id: "write_agent_config",
        description: "Update Infisical Agent configuration".to_string(),
        kind: OperationKind::WriteFile {
            path: config_path.clone(),
            content: agent_yaml_content,
            mode: Some(0o640),
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    ops.push(PlannedOperation {
        id: "systemd_restart",
        description: "Restart infisical-agent service".to_string(),
        kind: OperationKind::Shell {
            command: "systemctl".to_string(),
            args: vec!["restart".to_string(), "infisical-agent.service".to_string()],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    let report = crate::runner::executor::execute_plan(
        &ops,
        &runner,
        crate::core::elevation::PrivilegeContext::ELEVATED_OPS,
        opts.dry_run,
        true,
        crate::runner::IoMode::Buffered,
    );

    if !report
        .iter()
        .any(|r| r.status == crate::core::operation::OperationStatus::Failed)
        && !opts.dry_run
    {
        let (mut hcfg, _) = crate::config::load()
            .unwrap_or_else(|_| (crate::config::HeimdallConfig::default(), cfg_path.clone()));
        let bootstrap = hcfg
            .bootstrap
            .get_or_insert_with(crate::config::BootstrapConfig::default);
        if let Some(ref mut infisical) = bootstrap.infisical {
            infisical.folders = discovered_folders;
        }
        if let Err(e) = crate::config::save(&hcfg, &cfg_path) {
            eprintln!("warning: failed to persist heimdall config: {e}");
        }
    }

    let style = match opts.output {
        crate::cli::OutputFormat::Human => crate::output::Style::for_human(global.color),
        crate::cli::OutputFormat::Json => crate::output::Style::plain(),
    };

    match opts.output {
        crate::cli::OutputFormat::Human => {
            println!(
                "{}",
                crate::output::format_operation_report(
                    "heimdall service infisical sync",
                    &report,
                    &style
                )
            );
        }
        crate::cli::OutputFormat::Json => {
            let service_report = crate::features::service::ServiceReport {
                service: "infisical".to_string(),
                operations: report.clone(),
            };
            println!("{}", serde_json::to_string_pretty(&service_report)?);
        }
    }

    Ok(
        if report
            .iter()
            .any(|r| r.status == crate::core::operation::OperationStatus::Failed)
        {
            ExitStatus::Failure
        } else {
            ExitStatus::Success
        },
    )
}
