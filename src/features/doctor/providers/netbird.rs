use crate::core::doctor::DoctorContext;

use super::super::report::{CheckStatus, DoctorCheck};
use super::command_available;

pub fn contribute(ctx: &DoctorContext) -> Vec<DoctorCheck> {
    if !command_available(ctx.runner, ctx.io_mode, "netbird") {
        return vec![DoctorCheck {
            id: "bootstrap_netbird",
            description: "NetBird client",
            status: CheckStatus::Warn,
            detail: "netbird CLI not on PATH".to_string(),
        }];
    }

    match ctx
        .runner
        .run_with_env_io("netbird", &["status"], &[], ctx.io_mode)
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail_trim = stdout.trim().to_string();
            if !output.status.success() {
                return vec![DoctorCheck {
                    id: "bootstrap_netbird",
                    description: "NetBird client",
                    status: CheckStatus::Warn,
                    detail: format!(
                        "netbird status exited {}; stderr={}",
                        output.status,
                        String::from_utf8_lossy(&output.stderr).trim()
                    ),
                }];
            }
            let ok = netbird_connected(&stdout);
            vec![DoctorCheck {
                id: "bootstrap_netbird",
                description: "NetBird client",
                status: if ok {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Warn
                },
                detail: if detail_trim.is_empty() {
                    "netbird status produced no output".to_string()
                } else {
                    detail_trim
                },
            }]
        }
        Err(e) => vec![DoctorCheck {
            id: "bootstrap_netbird",
            description: "NetBird client",
            status: CheckStatus::Warn,
            detail: format!("could not run netbird status: {e:#}"),
        }],
    }
}

fn netbird_connected(stdout: &str) -> bool {
    let mut mgmt = false;
    let mut sig = false;
    for line in stdout.lines() {
        if line.contains("Management:") && line.contains("Connected") {
            mgmt = true;
        }
        if line.contains("Signal:") && line.contains("Connected") {
            sig = true;
        }
    }
    mgmt && sig
}
