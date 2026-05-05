use anyhow::{Result, bail};

use crate::core::operation::{OperationKind, PlannedOperation};

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
            description: "Ensure admin group exists".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "getent group {} >/dev/null || groupadd {}",
                        config.group, config.group
                    ),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        },
        PlannedOperation {
            id: "ensure_user",
            description: "Ensure admin user exists".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "id -u {} >/dev/null 2>&1 || useradd --create-home --shell /bin/bash --gid {} {}",
                        config.user, config.group, config.user
                    ),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        },
        PlannedOperation {
            id: "grant_sudo_access",
            description: "Add user to sudo/wheel group for administrator privileges".to_string(),
            kind: OperationKind::Shell {
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
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        },
        PlannedOperation {
            id: "ensure_ssh_dir",
            description: "Ensure .ssh directory and permissions".to_string(),
            kind: OperationKind::Shell {
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
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        },
    ];

    let authorized_keys_path = format!("/home/{}/.ssh/authorized_keys", config.user);
    let authorized_keys_tmp_path = format!("/home/{}/.ssh/.authorized_keys.tmp", config.user);
    operations.push(PlannedOperation {
        id: "ensure_authorized_keys_file",
        description: "Ensure authorized_keys file exists".to_string(),
        kind: OperationKind::Shell {
            command: "sudo".to_string(),
            args: vec!["touch".to_string(), authorized_keys_path.clone()],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });
    operations.push(PlannedOperation {
        id: "prepare_authorized_keys_temp",
        description: "Create temporary authorized_keys copy for atomic update".to_string(),
        kind: OperationKind::Shell {
            command: "sudo".to_string(),
            args: vec![
                "cp".to_string(),
                authorized_keys_path.clone(),
                authorized_keys_tmp_path.clone(),
            ],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });
    for key in &config.keys {
        operations.push(PlannedOperation {
            id: "append_authorized_key",
            description: "Install allowed SSH key in temporary file if missing".to_string(),
            kind: OperationKind::Shell {
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
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        });
    }
    operations.push(PlannedOperation {
        id: "promote_authorized_keys_temp",
        description: "Atomically replace authorized_keys with temporary file".to_string(),
        kind: OperationKind::Shell {
            command: "sudo".to_string(),
            args: vec![
                "mv".to_string(),
                authorized_keys_tmp_path,
                authorized_keys_path.clone(),
            ],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    operations.push(PlannedOperation {
        id: "set_authorized_keys_permissions",
        description: "Set authorized_keys file ownership and mode".to_string(),
        kind: OperationKind::Shell {
            command: "sudo".to_string(),
            args: vec![
                "chown".to_string(),
                format!("{}:{}", config.user, config.group),
                authorized_keys_path.clone(),
            ],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });
    operations.push(PlannedOperation {
        id: "chmod_authorized_keys",
        description: "Set authorized_keys mode".to_string(),
        kind: OperationKind::Shell {
            command: "sudo".to_string(),
            args: vec!["chmod".to_string(), "600".to_string(), authorized_keys_path],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    operations.push(PlannedOperation {
        id: "set_password",
        description: "Set user password via chpasswd".to_string(),
        kind: OperationKind::Shell {
            command: "sudo".to_string(),
            args: vec!["chpasswd".to_string()],
            env: vec![],
            stdin_input: Some(format!("{}:{}\n", config.user, config.password)),
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    if config.disable_root_login {
        operations.push(PlannedOperation {
            id: "disable_root_login",
            description: "Disable SSH root login in sshd_config".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec![
                    "sed".to_string(),
                    "-i.bak".to_string(),
                    "s/^#\\?PermitRootLogin.*/PermitRootLogin no/".to_string(),
                    "/etc/ssh/sshd_config".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: true,
            failure_is_warning: false,
            verify: None,
        });
    }

    if config.disable_password_auth {
        operations.push(PlannedOperation {
            id: "disable_password_auth",
            description: "Disable SSH password authentication in sshd_config".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec![
                    "sed".to_string(),
                    "-i.bak".to_string(),
                    "s/^#\\?PasswordAuthentication.*/PasswordAuthentication no/".to_string(),
                    "/etc/ssh/sshd_config".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: true,
            failure_is_warning: false,
            verify: None,
        });
    }

    if config.disable_root_login || config.disable_password_auth {
        operations.push(PlannedOperation {
            id: "validate_sshd_config",
            description: "Validate SSH daemon configuration".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec!["sshd".to_string(), "-t".to_string()],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: true,
            failure_is_warning: false,
            verify: None,
        });
        operations.push(PlannedOperation {
            id: "reload_sshd",
            description: "Reload SSH daemon".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec![
                    "systemctl".to_string(),
                    "reload".to_string(),
                    "sshd".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: true,
            failure_is_warning: false,
            verify: None,
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
        use crate::core::operation::OperationKind;
        let c = config();
        let plan = build_plan(&c).expect("plan should build");
        let ensure_user = plan
            .iter()
            .find(|op| op.id == "ensure_user")
            .expect("ensure_user operation must exist");
        if let OperationKind::Shell { args, .. } = &ensure_user.kind {
            assert!(args.join(" ").contains("id -u"));
        } else {
            panic!("expected Shell kind");
        }

        let append_key = plan
            .iter()
            .find(|op| op.id == "append_authorized_key")
            .expect("append_authorized_key operation must exist");
        if let OperationKind::Shell { args, .. } = &append_key.kind {
            assert!(args.join(" ").contains("grep -qxF"));
            assert!(args.join(" ").contains(".authorized_keys.tmp"));
        } else {
            panic!("expected Shell kind");
        }

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
        use crate::core::operation::OperationKind;
        let c = config();
        let plan = build_plan(&c).expect("plan should build");
        let op = plan
            .iter()
            .find(|op| op.id == "grant_sudo_access")
            .expect("grant_sudo_access operation must exist");
        if let OperationKind::Shell { args, .. } = &op.kind {
            let args_str = args.join(" ");
            assert!(
                args_str.contains("usermod -aG sudo"),
                "args must reference sudo group: {args_str}"
            );
            assert!(
                args_str.contains("usermod -aG wheel"),
                "args must reference wheel group: {args_str}"
            );
        } else {
            panic!("expected Shell kind");
        }
    }
}
