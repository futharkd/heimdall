use anyhow::{Result, bail};
use serde::Serialize;

use crate::runner::CommandRunner;

#[derive(Debug, Clone)]
pub struct BootstrapUserConfig {
    pub user: String,
    pub group: String,
    pub keys: Vec<String>,
    pub disable_root_login: bool,
    pub disable_password_auth: bool,
    pub dry_run: bool,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Planned,
    Skipped,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperationResult {
    pub id: &'static str,
    pub description: &'static str,
    pub status: OperationStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapUserReport {
    pub operations: Vec<OperationResult>,
}

impl BootstrapUserReport {
    pub fn has_failures(&self) -> bool {
        self.operations
            .iter()
            .any(|operation| operation.status == OperationStatus::Failed)
    }
}

#[derive(Debug, Clone)]
pub struct PlannedOperation {
    pub id: &'static str,
    pub description: &'static str,
    pub command: String,
    pub args: Vec<String>,
    pub requires_confirmation: bool,
}

pub fn validate_username(username: &str) -> Result<()> {
    if username.is_empty() {
        bail!("username must not be empty");
    }

    let first = username.as_bytes()[0] as char;
    if !(first.is_ascii_lowercase() || first == '_') {
        bail!("username must start with a lowercase letter or underscore");
    }

    if !username
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        bail!("username contains unsupported characters");
    }

    Ok(())
}

pub fn validate_ssh_key(key: &str) -> Result<()> {
    let mut parts = key.split_whitespace();
    let algo = parts.next().unwrap_or_default();
    let payload = parts.next().unwrap_or_default();
    if algo.is_empty() || payload.is_empty() {
        bail!("ssh key must contain algorithm and payload");
    }

    let supported = ["ssh-ed25519", "ssh-rsa", "ecdsa-sha2-nistp256"];
    if !supported.contains(&algo) {
        bail!("unsupported ssh key algorithm: {algo}");
    }

    Ok(())
}

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
    });
    operations.push(PlannedOperation {
        id: "chmod_authorized_keys",
        description: "Set authorized_keys mode",
        command: "sudo".to_string(),
        args: vec!["chmod".to_string(), "600".to_string(), authorized_keys_path],
        requires_confirmation: false,
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
        });
    }

    if config.disable_root_login || config.disable_password_auth {
        operations.push(PlannedOperation {
            id: "validate_sshd_config",
            description: "Validate SSH daemon configuration",
            command: "sudo".to_string(),
            args: vec!["sshd".to_string(), "-t".to_string()],
            requires_confirmation: true,
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
        });
    }

    Ok(operations)
}

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &BootstrapUserConfig,
    operations: &[PlannedOperation],
) -> BootstrapUserReport {
    let mut results = Vec::with_capacity(operations.len());

    for operation in operations {
        if operation.requires_confirmation && !config.confirmed {
            results.push(OperationResult {
                id: operation.id,
                description: operation.description,
                status: OperationStatus::Skipped,
                detail: "skipped because risky changes were not confirmed".to_string(),
            });
            continue;
        }

        if config.dry_run {
            results.push(OperationResult {
                id: operation.id,
                description: operation.description,
                status: OperationStatus::Planned,
                detail: format!(
                    "dry-run: {} {}",
                    operation.command,
                    operation.args.join(" ")
                ),
            });
            continue;
        }

        let arg_refs: Vec<&str> = operation.args.iter().map(String::as_str).collect();
        match runner.run(&operation.command, &arg_refs) {
            Ok(output) if output.status.success() => results.push(OperationResult {
                id: operation.id,
                description: operation.description,
                status: OperationStatus::Succeeded,
                detail: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            }),
            Ok(output) => {
                results.push(OperationResult {
                    id: operation.id,
                    description: operation.description,
                    status: OperationStatus::Failed,
                    detail: format!(
                        "exit status {}: {}",
                        output.status,
                        String::from_utf8_lossy(&output.stderr).trim()
                    ),
                });
                break;
            }
            Err(err) => {
                results.push(OperationResult {
                    id: operation.id,
                    description: operation.description,
                    status: OperationStatus::Failed,
                    detail: format!("failed to execute: {err}"),
                });
                break;
            }
        }
    }

    BootstrapUserReport {
        operations: results,
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{BootstrapUserConfig, OperationStatus, build_plan, execute_plan, validate_ssh_key};
    use crate::runner::CommandRunner;

    struct MockRunner {
        fail_after: Option<usize>,
        calls: std::sync::Mutex<usize>,
    }

    impl CommandRunner for MockRunner {
        fn run(&self, _program: &str, _args: &[&str]) -> Result<std::process::Output> {
            let mut guard = self.calls.lock().expect("lock");
            *guard += 1;
            if self.fail_after.is_some_and(|n| *guard >= n) {
                return Ok(std::process::Output {
                    status: std::os::unix::process::ExitStatusExt::from_raw(1 << 8),
                    stdout: vec![],
                    stderr: b"boom".to_vec(),
                });
            }
            Ok(std::process::Output {
                status: std::os::unix::process::ExitStatusExt::from_raw(0),
                stdout: b"ok".to_vec(),
                stderr: vec![],
            })
        }
    }

    fn config() -> BootstrapUserConfig {
        BootstrapUserConfig {
            user: "admin".to_string(),
            group: "admin".to_string(),
            keys: vec!["ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIabc123 user@host".to_string()],
            disable_root_login: true,
            disable_password_auth: false,
            dry_run: false,
            confirmed: true,
        }
    }

    #[test]
    fn rejects_invalid_key() {
        assert!(validate_ssh_key("not-a-key").is_err());
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
    fn execute_stops_on_failure() {
        let c = config();
        let plan = build_plan(&c).expect("plan should build");
        let runner = MockRunner {
            fail_after: Some(2),
            calls: std::sync::Mutex::new(0),
        };
        let report = execute_plan(&runner, &c, &plan);
        assert!(report.has_failures());
    }

    #[test]
    fn risky_steps_skipped_without_confirmation() {
        let mut c = config();
        c.confirmed = false;
        let plan = build_plan(&c).expect("plan should build");
        let runner = MockRunner {
            fail_after: None,
            calls: std::sync::Mutex::new(0),
        };
        let report = execute_plan(&runner, &c, &plan);
        assert!(
            report
                .operations
                .iter()
                .any(|op| op.status == OperationStatus::Skipped)
        );
    }
}
