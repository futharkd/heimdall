use anyhow::{Result, bail};

use crate::core::operation::PlannedOperation;

use super::input::BootstrapUserConfig;
use super::validate::{validate_ssh_key, validate_username};

pub fn build_plan(config: &BootstrapUserConfig) -> Result<Vec<PlannedOperation>> {
    validate_username(&config.user)?;
    validate_username(&config.group)?;

    if config.keys.is_empty() {
        bail!("at least one SSH key is required");
    }

    for key in &config.keys {
        validate_ssh_key(key)?;
    }

    let mut operations = vec![
        PlannedOperation {
            id: "ensure_group",
            description: "Ensure admin group exists",
            command: "sudo".to_string(),
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "getent group {} >/dev/null || groupadd {}",
                    config.group, config.group
                ),
            ],
            requires_confirmation: false,
            stdin_input: None,
        },
        PlannedOperation {
            id: "ensure_user",
            description: "Ensure admin user exists",
            command: "sudo".to_string(),
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "id -u {} >/dev/null 2>&1 || useradd --create-home --shell /bin/bash --gid {} {}",
                    config.user, config.group, config.user
                ),
            ],
            requires_confirmation: false,
            stdin_input: None,
        },
        PlannedOperation {
            id: "grant_sudo_access",
            description: "Add user to sudo/wheel group for administrator privileges",
            command: "sudo".to_string(),
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "getent group sudo >/dev/null 2>&1 && usermod -aG sudo {user}; \
                     getent group wheel >/dev/null 2>&1 && usermod -aG wheel {user}; true",
                    user = config.user
                ),
            ],
            requires_confirmation: false,
            stdin_input: None,
        },
        PlannedOperation {
            id: "ensure_ssh_dir",
            description: "Ensure .ssh directory and permissions",
            command: "sudo".to_string(),
            args: vec![
                "install".to_string(),
                "-d".to_string(),
                "-m".to_string(),
                "700".to_string(),
                "-o".to_string(),
                config.user.clone(),
                "-g".to_string(),
                config.group.clone(),
                format!("/home/{}/.ssh", config.user),
            ],
            requires_confirmation: false,
            stdin_input: None,
        },
    ];

    let authorized_keys_path = format!("/home/{}/.ssh/authorized_keys", config.user);
    let authorized_keys_tmp_path = format!("/home/{}/.ssh/.authorized_keys.tmp", config.user);
    operations.push(PlannedOperation {
        id: "ensure_authorized_keys_file",
        description: "Ensure authorized_keys file exists",
        command: "sudo".to_string(),
        args: vec!["touch".to_string(), authorized_keys_path.clone()],
        requires_confirmation: false,
        stdin_input: None,
    });
    operations.push(PlannedOperation {
        id: "prepare_authorized_keys_temp",
        description: "Create temporary authorized_keys copy for atomic update",
        command: "sudo".to_string(),
        args: vec![
            "cp".to_string(),
            authorized_keys_path.clone(),
            authorized_keys_tmp_path.clone(),
        ],
        requires_confirmation: false,
        stdin_input: None,
    });
    for key in &config.keys {
        operations.push(PlannedOperation {
            id: "append_authorized_key",
            description: "Install allowed SSH key in temporary file if missing",
            command: "sudo".to_string(),
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "grep -qxF '{}' {} || printf '%s\\n' '{}' >> {}",
                    key.replace('\'', "'\"'\"'"),
                    authorized_keys_tmp_path,
                    key.replace('\'', "'\"'\"'"),
                    authorized_keys_tmp_path
                ),
            ],
            requires_confirmation: false,
            stdin_input: None,
        });
    }
    operations.push(PlannedOperation {
        id: "promote_authorized_keys_temp",
        description: "Atomically replace authorized_keys with temporary file",
        command: "sudo".to_string(),
        args: vec![
            "mv".to_string(),
            authorized_keys_tmp_path,
            authorized_keys_path.clone(),
        ],
        requires_confirmation: false,
        stdin_input: None,
    });

    operations.push(PlannedOperation {
        id: "set_authorized_keys_permissions",
        description: "Set authorized_keys file ownership and mode",
        command: "sudo".to_string(),
        args: vec![
            "chown".to_string(),
            format!("{}:{}", config.user, config.group),
            authorized_keys_path.clone(),
        ],
        requires_confirmation: false,
        stdin_input: None,
    });
    operations.push(PlannedOperation {
        id: "chmod_authorized_keys",
        description: "Set authorized_keys mode",
        command: "sudo".to_string(),
        args: vec!["chmod".to_string(), "600".to_string(), authorized_keys_path],
        requires_confirmation: false,
        stdin_input: None,
    });

    operations.push(PlannedOperation {
        id: "set_password",
        description: "Set user password via chpasswd",
        command: "sudo".to_string(),
        args: vec!["chpasswd".to_string()],
        requires_confirmation: false,
        stdin_input: Some(format!("{}:{}\n", config.user, config.password)),
    });

    if config.disable_root_login {
        operations.push(PlannedOperation {
            id: "disable_root_login",
            description: "Disable SSH root login in sshd_config",
            command: "sudo".to_string(),
            args: vec![
                "sed".to_string(),
                "-i.bak".to_string(),
                "s/^#\\?PermitRootLogin.*/PermitRootLogin no/".to_string(),
                "/etc/ssh/sshd_config".to_string(),
            ],
            requires_confirmation: true,
            stdin_input: None,
        });
    }

    if config.disable_password_auth {
        operations.push(PlannedOperation {
            id: "disable_password_auth",
            description: "Disable SSH password authentication in sshd_config",
            command: "sudo".to_string(),
            args: vec![
                "sed".to_string(),
                "-i.bak".to_string(),
                "s/^#\\?PasswordAuthentication.*/PasswordAuthentication no/".to_string(),
                "/etc/ssh/sshd_config".to_string(),
            ],
            requires_confirmation: true,
            stdin_input: None,
        });
    }

    if config.disable_root_login || config.disable_password_auth {
        operations.push(PlannedOperation {
            id: "validate_sshd_config",
            description: "Validate SSH daemon configuration",
            command: "sudo".to_string(),
            args: vec!["sshd".to_string(), "-t".to_string()],
            requires_confirmation: true,
            stdin_input: None,
        });
        operations.push(PlannedOperation {
            id: "reload_sshd",
            description: "Reload SSH daemon",
            command: "sudo".to_string(),
            args: vec![
                "systemctl".to_string(),
                "reload".to_string(),
                "sshd".to_string(),
            ],
            requires_confirmation: true,
            stdin_input: None,
        });
    }

    Ok(operations)
}

#[cfg(test)]
mod tests {
    use super::build_plan;
    use crate::features::bootstrap::user::input::BootstrapUserConfig;

    fn config() -> BootstrapUserConfig {
        BootstrapUserConfig {
            user: "admin".to_string(),
            group: "admin".to_string(),
            keys: vec!["ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIabc123 user@host".to_string()],
            password: "testpass".to_string(),
            disable_root_login: true,
            disable_password_auth: false,
            dry_run: false,
            confirmed: true,
        }
    }

    #[test]
    fn plan_requires_key() {
        let mut c = config();
        c.keys.clear();
        assert!(build_plan(&c).is_err());
    }

    #[test]
    fn plan_uses_idempotent_user_and_key_operations() {
        let c = config();
        let plan = build_plan(&c).expect("plan should build");
        let ensure_user = plan
            .iter()
            .find(|op| op.id == "ensure_user")
            .expect("ensure_user operation must exist");
        assert!(ensure_user.args.join(" ").contains("id -u"));

        let append_key = plan
            .iter()
            .find(|op| op.id == "append_authorized_key")
            .expect("append_authorized_key operation must exist");
        assert!(append_key.args.join(" ").contains("grep -qxF"));
        assert!(append_key.args.join(" ").contains(".authorized_keys.tmp"));

        assert!(
            plan.iter()
                .any(|op| op.id == "prepare_authorized_keys_temp")
        );
        assert!(
            plan.iter()
                .any(|op| op.id == "promote_authorized_keys_temp")
        );
    }

    #[test]
    fn plan_grants_sudo_access() {
        let c = config();
        let plan = build_plan(&c).expect("plan should build");
        let op = plan
            .iter()
            .find(|op| op.id == "grant_sudo_access")
            .expect("grant_sudo_access operation must exist");
        let args_str = op.args.join(" ");
        assert!(
            args_str.contains("usermod -aG sudo"),
            "args must reference sudo group: {args_str}"
        );
        assert!(
            args_str.contains("usermod -aG wheel"),
            "args must reference wheel group: {args_str}"
        );
    }
}
