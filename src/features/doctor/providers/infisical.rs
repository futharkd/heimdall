use crate::core::doctor::DoctorContext;
use crate::runner::IoMode;

use super::super::report::{CheckStatus, DoctorCheck};
use super::command_available;

pub fn contribute(ctx: &DoctorContext) -> Vec<DoctorCheck> {
    let on_path = command_available(ctx.runner, ctx.io_mode, "infisical");
    let mut checks = vec![DoctorCheck {
        id: "bootstrap_infisical",
        description: "Infisical CLI",
        status: if on_path {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        detail: if on_path {
            "infisical CLI found on PATH".to_string()
        } else {
            "infisical not found on PATH".to_string()
        },
    }];
    checks.extend(infisical_agent_service_check(ctx));
    checks
}

fn infisical_agent_service_check(ctx: &DoctorContext) -> Vec<DoctorCheck> {
    let mut mode = ctx.io_mode;
    if matches!(mode, IoMode::LiveTee) {
        mode = IoMode::Buffered;
    }
    match ctx.runner.run_with_env_io(
        "systemctl",
        &["is-active", "infisical-agent.service"],
        &[],
        mode,
    ) {
        Ok(output) => {
            let active = output.status.success()
                && String::from_utf8_lossy(&output.stdout).trim() == "active";
            vec![DoctorCheck {
                id: "bootstrap_infisical_agent",
                description: "Infisical agent service",
                status: if active {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Warn
                },
                detail: format!(
                    "systemctl is-active infisical-agent.service → {}",
                    String::from_utf8_lossy(&output.stdout).trim()
                ),
            }]
        }
        Err(e) => vec![DoctorCheck {
            id: "bootstrap_infisical_agent",
            description: "Infisical agent service",
            status: CheckStatus::Warn,
            detail: format!("could not check infisical-agent.service: {e:#}"),
        }],
    }
}
