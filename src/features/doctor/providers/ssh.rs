use std::path::Path;

use crate::core::doctor::DoctorContext;
use crate::runner::read::read_file_with_escalation;

use super::super::report::{CheckStatus, DoctorCheck};

const PATH_SSHD: &str = "/etc/ssh/sshd_config";

pub fn contribute(ctx: &DoctorContext) -> Vec<DoctorCheck> {
    let path = Path::new(PATH_SSHD);
    match read_file_with_escalation(ctx.runner, path, ctx.io_mode) {
        Ok(content) => {
            let interesting: Vec<&str> = content
                .lines()
                .filter(|line| {
                    let t = line.trim_start();
                    if t.is_empty() || t.starts_with('#') {
                        return false;
                    }
                    t.starts_with("Port ")
                        || t.starts_with("PermitRootLogin")
                        || t.starts_with("PasswordAuthentication")
                })
                .take(12)
                .collect();
            let detail = if interesting.is_empty() {
                format!(
                    "read {PATH_SSHD}; no uncommented Port/PermitRootLogin/PasswordAuthentication lines matched"
                )
            } else {
                interesting.join("\n")
            };
            vec![DoctorCheck {
                id: "harden_ssh",
                description: "SSH server config (sshd_config excerpts)",
                status: CheckStatus::Pass,
                detail,
            }]
        }
        Err(e) => vec![DoctorCheck {
            id: "harden_ssh",
            description: "SSH server config (sshd_config excerpts)",
            status: CheckStatus::Warn,
            detail: format!("cannot read {PATH_SSHD}: {e}"),
        }],
    }
}
