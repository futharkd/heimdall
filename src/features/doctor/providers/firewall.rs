use crate::core::doctor::DoctorContext;
use crate::runner::IoMode;

use super::super::report::{CheckStatus, DoctorCheck};

pub fn contribute(ctx: &DoctorContext) -> Vec<DoctorCheck> {
    let mut mode = ctx.io_mode;
    if matches!(mode, IoMode::LiveTee) {
        mode = IoMode::Buffered;
    }
    match ctx
        .runner
        .run_with_env_io("systemctl", &["is-active", "firewalld"], &[], mode)
    {
        Ok(output) => {
            let active = output.status.success()
                && String::from_utf8_lossy(&output.stdout).trim() == "active";
            vec![DoctorCheck {
                id: "harden_firewalld",
                description: "firewalld service",
                status: if active {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Warn
                },
                detail: format!(
                    "systemctl is-active firewalld → {}",
                    String::from_utf8_lossy(&output.stdout).trim()
                ),
            }]
        }
        Err(e) => vec![DoctorCheck {
            id: "harden_firewalld",
            description: "firewalld service",
            status: CheckStatus::Warn,
            detail: format!("could not check firewalld: {e:#}"),
        }],
    }
}
