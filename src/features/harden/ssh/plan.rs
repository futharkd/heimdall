use super::input::HardenSshConfig;
use super::validate::validate_port;
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SshPlannedOperation {
    pub id: String,
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
    pub failure_is_warning: bool,
}

pub fn build_plan(config: &HardenSshConfig) -> Result<Vec<SshPlannedOperation>> {
    let mut operations = Vec::new();

    if let Some(new_port) = config.new_port {
        validate_port(new_port)?;

        if new_port != config.current_port {
            operations.push(SshPlannedOperation {
                id: "backup_sshd_config".to_string(),
                description: "Backup sshd_config".to_string(),
                command: "cp".to_string(),
                args: vec![
                    "/etc/ssh/sshd_config".to_string(),
                    "/etc/ssh/sshd_config.heimdall.bak".to_string(),
                ],
                failure_is_warning: false,
            });

            operations.push(SshPlannedOperation {
                id: "change_ssh_port".to_string(),
                description: format!("Change SSH port to {}", new_port),
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
                failure_is_warning: false,
            });

            operations.push(SshPlannedOperation {
                id: "ensure_port_directive".to_string(),
                description: format!("Ensure Port {} directive present", new_port),
                command: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    format!(
                        "grep -qE '^Port [[:space:]]*{}' /etc/ssh/sshd_config || printf '\\nPort {}\\n' >> /etc/ssh/sshd_config",
                        new_port, new_port
                    ),
                ],
                failure_is_warning: false,
            });

            operations.push(SshPlannedOperation {
                id: "verify_port_in_config".to_string(),
                description: format!("Verify port {} set in sshd_config", new_port),
                command: "grep".to_string(),
                args: vec![
                    "-qE".to_string(),
                    format!("^Port[[:space:]]+{}", new_port),
                    "/etc/ssh/sshd_config".to_string(),
                ],
                failure_is_warning: false,
            });

            operations.push(SshPlannedOperation {
                id: "validate_sshd_config".to_string(),
                description: "Validate sshd_config".to_string(),
                command: "sshd".to_string(),
                args: vec!["-t".to_string()],
                failure_is_warning: false,
            });

            operations.push(SshPlannedOperation {
                id: "selinux_allow_ssh_port".to_string(),
                description: format!("Label port {} as ssh_port_t in SELinux policy", new_port),
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
                failure_is_warning: false,
            });

            operations.push(SshPlannedOperation {
                id: "reload_sshd".to_string(),
                description: "Reload SSH service".to_string(),
                command: "systemctl".to_string(),
                args: vec!["reload-or-restart".to_string(), "sshd".to_string()],
                failure_is_warning: false,
            });

            operations.push(SshPlannedOperation {
                id: "verify_ssh_listening".to_string(),
                description: format!("Verify SSH listening on port {}", new_port),
                command: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    format!("ss -tlnp 2>/dev/null | grep -q :{} || netstat -tlnp 2>/dev/null | grep -q :{}", new_port, new_port),
                ],
                failure_is_warning: false,
            });
        }
    }

    if config.disable_root_login {
        operations.push(SshPlannedOperation {
            id: "disable_root_login".to_string(),
            description: "Disable root login".to_string(),
            command: "sed".to_string(),
            args: vec![
                "-E".to_string(),
                "-i".to_string(),
                "s/^[[:space:]]*#?[[:space:]]*PermitRootLogin[[:space:]]+.*/PermitRootLogin no/"
                    .to_string(),
                "/etc/ssh/sshd_config".to_string(),
            ],
            failure_is_warning: false,
        });

        operations.push(SshPlannedOperation {
            id: "verify_root_login_disabled".to_string(),
            description: "Verify root login disabled".to_string(),
            command: "grep".to_string(),
            args: vec![
                "-qE".to_string(),
                "^PermitRootLogin[[:space:]]+no".to_string(),
                "/etc/ssh/sshd_config".to_string(),
            ],
            failure_is_warning: false,
        });
    }

    if config.disable_password_auth {
        operations.push(SshPlannedOperation {
            id: "disable_password_auth".to_string(),
            description: "Disable password authentication".to_string(),
            command: "sed".to_string(),
            args: vec![
                "-E".to_string(),
                "-i".to_string(),
                "s/^[[:space:]]*#?[[:space:]]*PasswordAuthentication[[:space:]]+.*/PasswordAuthentication no/".to_string(),
                "/etc/ssh/sshd_config".to_string(),
            ],
            failure_is_warning: false,
        });

        operations.push(SshPlannedOperation {
            id: "verify_password_auth_disabled".to_string(),
            description: "Verify password authentication disabled".to_string(),
            command: "grep".to_string(),
            args: vec![
                "-qE".to_string(),
                "^PasswordAuthentication[[:space:]]+no".to_string(),
                "/etc/ssh/sshd_config".to_string(),
            ],
            failure_is_warning: false,
        });
    }

    if (config.disable_root_login || config.disable_password_auth) && operations.len() > 1 {
        operations.push(SshPlannedOperation {
            id: "validate_final".to_string(),
            description: "Final sshd_config validation".to_string(),
            command: "sshd".to_string(),
            args: vec!["-t".to_string()],
            failure_is_warning: false,
        });
    }

    Ok(operations)
}
