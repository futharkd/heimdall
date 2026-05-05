use std::fs;

use super::super::report::{CheckStatus, DoctorCheck};

const PATH_SSHD: &str = "/etc/ssh/sshd_config";

pub fn contribute() -> Vec<DoctorCheck> {
    match fs::read_to_string(PATH_SSHD) {
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
