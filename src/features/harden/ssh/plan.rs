use crate::core::operation::{OperationKind, PlannedOperation, VerifyStep};
use anyhow::Result;

use super::input::HardenSshConfig;
use super::validate::validate_port;

const SELINUX_UTILS_PKG: &str = "policycoreutils-python-utils";

pub fn build_plan(config: &HardenSshConfig) -> Result<Vec<PlannedOperation>> {
    let mut operations = Vec::new();

    if let Some(new_port) = config.new_port {
        validate_port(new_port)?;

        if new_port != config.current_port {
            operations.push(PlannedOperation {
                id: "backup_sshd_config",
                description: "Backup sshd_config".to_string(),
                kind: OperationKind::Shell {
                    command: "cp".to_string(),
                    args: vec![
                        "/etc/ssh/sshd_config".to_string(),
                        "/etc/ssh/sshd_config.heimdall.bak".to_string(),
                    ],
                    env: vec![],
                    stdin_input: None,
                },
                requires_confirmation: false,
                failure_is_warning: false,
                verify: None,
            });

            operations.push(PlannedOperation {
                id: "change_ssh_port",
                description: format!("Change SSH port to {}", new_port),
                kind: OperationKind::Shell {
                    command: "sed".to_string(),
                    args: vec![
                        "-E".to_string(),
                        "-i".to_string(),
                        format!(
                            "s/^[[:space:]]*#?[[:space:]]*Port[[:space:]]+.*/Port {}/",
                            new_port
                        ),
                        "/etc/ssh/sshd_config".to_string(),
                    ],
                    env: vec![],
                    stdin_input: None,
                },
                requires_confirmation: false,
                failure_is_warning: false,
                verify: None,
            });

            let port_verify = VerifyStep {
                description: format!("verify port {} set in sshd_config", new_port),
                command: "grep".to_string(),
                args: vec![
                    "-qE".to_string(),
                    format!("^Port[[:space:]]+{}", new_port),
                    "/etc/ssh/sshd_config".to_string(),
                ],
            };

            operations.push(PlannedOperation {
                id: "ensure_port_directive",
                description: format!("Ensure Port {} directive present", new_port),
                kind: OperationKind::Shell {
                    command: "sh".to_string(),
                    args: vec![
                        "-c".to_string(),
                        format!(
                            "grep -qE '^Port [[:space:]]*{}' /etc/ssh/sshd_config || printf '\\nPort {}\\n' >> /etc/ssh/sshd_config",
                            new_port, new_port
                        ),
                    ],
                    env: vec![],
                    stdin_input: None,
                },
                requires_confirmation: false,
                failure_is_warning: false,
                verify: Some(port_verify),
            });

            operations.push(PlannedOperation {
                id: "validate_sshd_config",
                description: "Validate sshd_config".to_string(),
                kind: OperationKind::Shell {
                    command: "sshd".to_string(),
                    args: vec!["-t".to_string()],
                    env: vec![],
                    stdin_input: None,
                },
                requires_confirmation: false,
                failure_is_warning: false,
                verify: None,
            });

            operations.push(PlannedOperation {
                id: "ensure_semanage_package",
                description: &format!(
                    "Install {} if missing (provides semanage)",
                    SELINUX_UTILS_PKG
                ),
                kind: OperationKind::EnsurePackage {
                    package: SELINUX_UTILS_PKG.to_string(),
                },
                requires_confirmation: false,
                failure_is_warning: false,
                verify: None,
            });

            operations.push(PlannedOperation {
                id: "selinux_allow_ssh_port",
                description: &format!("Label port {} as ssh_port_t in SELinux policy", new_port),
                kind: OperationKind::Shell {
                    command: "sh".to_string(),
                    args: vec![
                        "-c".to_string(),
                        format!(
                            "! command -v semanage >/dev/null 2>&1 \
                             || semanage port -l | grep -q 'ssh_port_t.*tcp.*\\b{0}\\b' \
                             || semanage port -a -t ssh_port_t -p tcp {0}",
                            new_port
                        ),
                    ],
                    env: vec![],
                    stdin_input: None,
                },
                requires_confirmation: false,
                failure_is_warning: false,
                verify: None,
            });

            let listen_verify = VerifyStep {
                description: &format!("verify SSH listening on port {}", new_port),
                command: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    format!("ss -tlnp 2>/dev/null | grep -q :{} || netstat -tlnp 2>/dev/null | grep -q :{}", new_port, new_port),
                ],
            };

            operations.push(PlannedOperation {
                id: "reload_sshd",
                description: "Reload SSH service".to_string(),
                kind: OperationKind::Shell {
                    command: "systemctl".to_string(),
                    args: vec!["reload-or-restart".to_string(), "sshd".to_string()],
                    env: vec![],
                    stdin_input: None,
                },
                requires_confirmation: false,
                failure_is_warning: false,
                verify: Some(listen_verify),
            });
        }
    }

    if config.disable_root_login {
        let root_verify = VerifyStep {
            description: "verify root login disabled",
            command: "grep".to_string(),
            args: vec![
                "-qE".to_string(),
                "^PermitRootLogin[[:space:]]+no".to_string(),
                "/etc/ssh/sshd_config".to_string(),
            ],
        };

        operations.push(PlannedOperation {
            id: "disable_root_login",
            description: "Disable root login".to_string(),
            kind: OperationKind::Shell {
                command: "sed".to_string(),
                args: vec![
                    "-E".to_string(),
                    "-i".to_string(),
                    "s/^[[:space:]]*#?[[:space:]]*PermitRootLogin[[:space:]]+.*/PermitRootLogin no/"
                        .to_string(),
                    "/etc/ssh/sshd_config".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: Some(root_verify),
        });
    }

    if config.disable_password_auth {
        let pwd_verify = VerifyStep {
            description: "verify password authentication disabled",
            command: "grep".to_string(),
            args: vec![
                "-qE".to_string(),
                "^PasswordAuthentication[[:space:]]+no".to_string(),
                "/etc/ssh/sshd_config".to_string(),
            ],
        };

        operations.push(PlannedOperation {
            id: "disable_password_auth",
            description: "Disable password authentication".to_string(),
            kind: OperationKind::Shell {
                command: "sed".to_string(),
                args: vec![
                    "-E".to_string(),
                    "-i".to_string(),
                    "s/^[[:space:]]*#?[[:space:]]*PasswordAuthentication[[:space:]]+.*/PasswordAuthentication no/".to_string(),
                    "/etc/ssh/sshd_config".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: Some(pwd_verify),
        });
    }

    if (config.disable_root_login || config.disable_password_auth) && operations.len() > 1 {
        operations.push(PlannedOperation {
            id: "validate_final",
            description: "Final sshd_config validation".to_string(),
            kind: OperationKind::Shell {
                command: "sshd".to_string(),
                args: vec!["-t".to_string()],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        });
    }

    Ok(operations)
}

#[cfg(test)]
mod tests {
    use super::build_plan;
    use crate::features::harden::ssh::input::HardenSshConfig;

    #[test]
    fn plan_contains_expected_ops_with_verify_steps() {
        let cfg = HardenSshConfig {
            new_port: Some(2222),
            current_port: 22,
            disable_root_login: true,
            disable_password_auth: true,
            dry_run: false,
            assume_yes: false,
        };
        let plan = build_plan(&cfg).expect("plan");

        // Check that key ops exist
        assert!(plan.iter().any(|o| o.id == "ensure_port_directive"));
        assert!(plan.iter().any(|o| o.id == "disable_root_login"));
        assert!(plan.iter().any(|o| o.id == "disable_password_auth"));

        // Verify ops should be integrated, not standalone
        assert!(!plan.iter().any(|o| o.id == "verify_port_in_config"));
        assert!(!plan.iter().any(|o| o.id == "verify_root_login_disabled"));
        assert!(!plan.iter().any(|o| o.id == "verify_password_auth_disabled"));
        assert!(!plan.iter().any(|o| o.id == "verify_ssh_listening"));

        // Cross-cutting validates should remain standalone
        assert!(plan.iter().any(|o| o.id == "validate_sshd_config"));
        assert!(plan.iter().any(|o| o.id == "validate_final"));
    }
}
