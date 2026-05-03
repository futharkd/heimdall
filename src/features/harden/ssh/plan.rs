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
                    "-i".to_string(),
                    format!("s/^#?Port .*/Port {}/", new_port),
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
                id: "reload_sshd".to_string(),
                description: "Reload SSH service".to_string(),
                command: "systemctl".to_string(),
                args: vec!["reload".to_string(), "sshd".to_string()],
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
                "-i".to_string(),
                "s/^#?PermitRootLogin .*/PermitRootLogin no/".to_string(),
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
                "-i".to_string(),
                "s/^#?PasswordAuthentication .*/PasswordAuthentication no/".to_string(),
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
