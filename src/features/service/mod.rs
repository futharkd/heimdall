pub mod infisical;
pub mod komodo;
pub mod netbird;

use std::path::PathBuf;

use crate::core::elevation::PrivilegeContext;
use crate::core::operation::{OperationKind, OperationResult, OperationStatus, PlannedOperation};
use crate::output::{Style, execution_footer_line, format_operation_report};
use crate::runner::{IoMode, LocalRunner, executor};
use crate::runtime::ExitStatus;

#[derive(Debug, Clone, Copy)]
pub enum ServiceKind {
    Komodo,
    Infisical,
    Netbird,
}

#[derive(Debug, Clone, Copy)]
pub enum ServiceActionKind {
    Status,
    Start,
    Stop,
    Restart,
}

#[derive(Debug, Clone)]
pub enum ServiceBackend {
    DockerCompose {
        compose_file: PathBuf,
        project_name: String,
        services: Vec<String>,
    },
    Systemd {
        unit_name: String,
    },
}

#[derive(Debug, Clone)]
pub struct ServiceActionPlan {
    pub kind: ServiceKind,
    pub action: ServiceActionKind,
    pub backend: ServiceBackend,
}

pub fn plan_service_actions(plan: ServiceActionPlan) -> Vec<PlannedOperation> {
    match plan.backend {
        ServiceBackend::DockerCompose {
            compose_file,
            project_name,
            services,
        } => plan_docker_compose(plan.kind, plan.action, compose_file, project_name, services),
        ServiceBackend::Systemd { unit_name } => plan_systemd(plan.kind, plan.action, unit_name),
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ServiceReport {
    pub service: String,
    pub operations: Vec<OperationResult>,
}

impl ServiceReport {
    pub fn has_failures(&self) -> bool {
        self.operations
            .iter()
            .any(|op| op.status == OperationStatus::Failed)
    }
}

pub fn execute_and_print(
    service_name: &str,
    operations: Vec<PlannedOperation>,
    output: crate::cli::OutputFormat,
    dry_run: bool,
    global: &crate::cli::GlobalOpts,
) -> anyhow::Result<ExitStatus> {
    let runner = LocalRunner;
    let live_execution = matches!(output, crate::cli::OutputFormat::Human) && !dry_run;
    let io_mode = if live_execution {
        IoMode::LiveTee
    } else {
        IoMode::Buffered
    };
    let results = executor::execute_plan(
        &operations,
        &runner,
        PrivilegeContext::ELEVATED_OPS,
        dry_run,
        true,
        io_mode,
    );
    let report = ServiceReport {
        service: service_name.to_string(),
        operations: results,
    };

    let style = match output {
        crate::cli::OutputFormat::Human => Style::for_human(global.color),
        crate::cli::OutputFormat::Json => Style::plain(),
    };

    match output {
        crate::cli::OutputFormat::Human if live_execution => {
            println!("{}", execution_footer_line(&report.operations))
        }
        crate::cli::OutputFormat::Human => {
            println!(
                "{}",
                format_operation_report(
                    &format!("heimdall service {}", service_name),
                    &report.operations,
                    &style
                )
            )
        }
        crate::cli::OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
    }

    Ok(if report.has_failures() {
        ExitStatus::Failure
    } else {
        ExitStatus::Success
    })
}

fn plan_docker_compose(
    kind: ServiceKind,
    action: ServiceActionKind,
    compose_file: PathBuf,
    project_name: String,
    services: Vec<String>,
) -> Vec<PlannedOperation> {
    let mut args = vec![
        "compose".to_string(),
        "-f".to_string(),
        compose_file.display().to_string(),
        "-p".to_string(),
        project_name,
    ];

    let (id, description, verb) = match action {
        ServiceActionKind::Status => (
            service_id(kind, "status"),
            service_description(kind, "Show service status"),
            "ps",
        ),
        ServiceActionKind::Start => (
            service_id(kind, "start"),
            service_description(kind, "Start service"),
            "up",
        ),
        ServiceActionKind::Stop => (
            service_id(kind, "stop"),
            service_description(kind, "Stop service"),
            "stop",
        ),
        ServiceActionKind::Restart => (
            service_id(kind, "restart"),
            service_description(kind, "Restart service"),
            "restart",
        ),
    };

    match action {
        ServiceActionKind::Status => {
            args.push(verb.to_string());
        }
        ServiceActionKind::Start => {
            args.push(verb.to_string());
            args.push("-d".to_string());
        }
        ServiceActionKind::Stop | ServiceActionKind::Restart => {
            args.push(verb.to_string());
        }
    }

    args.extend(services);

    vec![PlannedOperation {
        id,
        description,
        kind: OperationKind::Shell {
            command: "docker".to_string(),
            args,
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    }]
}

fn plan_systemd(
    kind: ServiceKind,
    action: ServiceActionKind,
    unit_name: String,
) -> Vec<PlannedOperation> {
    let (verb, id_suffix, description) = match action {
        ServiceActionKind::Status => (
            "is-active",
            "status",
            "Check systemd unit status".to_string(),
        ),
        ServiceActionKind::Start => ("start", "start", "Start systemd unit".to_string()),
        ServiceActionKind::Stop => ("stop", "stop", "Stop systemd unit".to_string()),
        ServiceActionKind::Restart => ("restart", "restart", "Restart systemd unit".to_string()),
    };

    vec![PlannedOperation {
        id: service_id(kind, id_suffix),
        description,
        kind: OperationKind::Shell {
            command: "systemctl".to_string(),
            args: vec![verb.to_string(), unit_name],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    }]
}

fn service_id(kind: ServiceKind, suffix: &'static str) -> &'static str {
    match kind {
        ServiceKind::Komodo => match suffix {
            "status" => "service_komodo_status",
            "start" => "service_komodo_start",
            "stop" => "service_komodo_stop",
            "restart" => "service_komodo_restart",
            _ => "service_komodo",
        },
        ServiceKind::Infisical => match suffix {
            "status" => "service_infisical_status",
            "start" => "service_infisical_start",
            "stop" => "service_infisical_stop",
            "restart" => "service_infisical_restart",
            _ => "service_infisical",
        },
        ServiceKind::Netbird => match suffix {
            "status" => "service_netbird_status",
            "start" => "service_netbird_start",
            "stop" => "service_netbird_stop",
            "restart" => "service_netbird_restart",
            _ => "service_netbird",
        },
    }
}

fn service_description(kind: ServiceKind, base: &str) -> String {
    let service = match kind {
        ServiceKind::Komodo => "Komodo",
        ServiceKind::Infisical => "Infisical",
        ServiceKind::Netbird => "Netbird",
    };
    format!("{base} for {service}")
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::cli::{Cli, Command, OutputFormat, ServiceTarget};

    use super::*;

    #[test]
    fn cli_parses_service_komodo_status() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "service",
            "komodo",
            "status",
            "--mode",
            "core",
            "--dry-run",
            "--output",
            "json",
        ])
        .expect("cli parses");

        let Command::Service(service) = parsed.command else {
            panic!("expected service command");
        };
        let ServiceTarget::Komodo(komodo) = service.target else {
            panic!("expected komodo service target");
        };
        assert!(komodo.dry_run);
        assert!(matches!(komodo.output, OutputFormat::Json));
    }

    #[test]
    fn cli_parses_service_netbird_restart() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "service",
            "netbird",
            "restart",
            "--unit-name",
            "netbird",
        ])
        .expect("cli parses");

        let Command::Service(service) = parsed.command else {
            panic!("expected service command");
        };
        let ServiceTarget::Netbird(netbird) = service.target else {
            panic!("expected netbird service target");
        };
        assert_eq!(netbird.unit_name, "netbird");
    }

    #[test]
    fn planner_builds_docker_compose_start() {
        let plan = ServiceActionPlan {
            kind: ServiceKind::Komodo,
            action: ServiceActionKind::Start,
            backend: ServiceBackend::DockerCompose {
                compose_file: PathBuf::from("/etc/heimdall/komodo/docker-compose.yml"),
                project_name: "komodo".to_string(),
                services: vec!["core".to_string()],
            },
        };
        let ops = plan_service_actions(plan);
        assert_eq!(ops.len(), 1);
        let OperationKind::Shell { command, args, .. } = &ops[0].kind else {
            panic!("expected shell op");
        };
        assert_eq!(command, "docker");
        assert!(args.iter().any(|a| a == "up"));
        assert!(args.iter().any(|a| a == "-d"));
        assert!(args.iter().any(|a| a == "core"));
    }

    #[test]
    fn planner_builds_systemd_status() {
        let plan = ServiceActionPlan {
            kind: ServiceKind::Infisical,
            action: ServiceActionKind::Status,
            backend: ServiceBackend::Systemd {
                unit_name: "infisical-agent.service".to_string(),
            },
        };
        let ops = plan_service_actions(plan);
        assert_eq!(ops.len(), 1);
        let OperationKind::Shell { command, args, .. } = &ops[0].kind else {
            panic!("expected shell op");
        };
        assert_eq!(command, "systemctl");
        assert_eq!(args[0], "is-active");
        assert_eq!(args[1], "infisical-agent.service");
    }
}
