use crate::core::operation::{OperationResult, OperationStatus};
use crate::runner::{CommandRunner, IoMode};

use super::input::BootstrapK3sConfig;
use super::plan::K3sPlannedOperation;
use super::report::BootstrapK3sReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &BootstrapK3sConfig,
    operations: &[K3sPlannedOperation],
    io_mode: IoMode,
) -> BootstrapK3sReport {
    let mut results = Vec::with_capacity(operations.len());

    for operation in operations {
        if config.dry_run {
            results.push(OperationResult {
                id: operation.id,
                description: operation.description,
                status: OperationStatus::Planned,
                detail: format_dry_run_detail(operation),
            });
            continue;
        }

        let arg_refs: Vec<&str> = operation.args.iter().map(String::as_str).collect();
        let env_refs: Vec<(&str, &str)> = operation
            .env
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let outcome = if operation.env.is_empty() {
            runner.run_with_env_io(&operation.command, &arg_refs, &[], io_mode)
        } else {
            runner.run_with_env_io(&operation.command, &arg_refs, &env_refs, io_mode)
        };

        match outcome {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let mut detail = stdout.trim().to_string();
                if detail.is_empty() {
                    detail = stderr.trim().to_string();
                }
                if operation.id == "k3s_kubectl_get_nodes" && !kubectl_nodes_name_output_ok(&detail)
                {
                    results.push(OperationResult {
                        id: operation.id,
                        description: operation.description,
                        status: OperationStatus::Failed,
                        detail: format!(
                            "sudo k3s kubectl succeeded but reported no nodes (expected `node/...` lines): {detail}"
                        ),
                    });
                    break;
                }
                results.push(OperationResult {
                    id: operation.id,
                    description: operation.description,
                    status: OperationStatus::Succeeded,
                    detail,
                });
            }
            Ok(output) => {
                let detail = format_command_failure(&output);
                if operation.failure_is_warning {
                    results.push(OperationResult {
                        id: operation.id,
                        description: operation.description,
                        status: OperationStatus::Skipped,
                        detail: format!("warning (non-fatal): {detail}"),
                    });
                    continue;
                }
                results.push(OperationResult {
                    id: operation.id,
                    description: operation.description,
                    status: OperationStatus::Failed,
                    detail,
                });
                break;
            }
            Err(err) => {
                let detail = format!("failed to execute: {err}");
                if operation.failure_is_warning {
                    results.push(OperationResult {
                        id: operation.id,
                        description: operation.description,
                        status: OperationStatus::Skipped,
                        detail: format!("warning (non-fatal): {detail}"),
                    });
                    continue;
                }
                results.push(OperationResult {
                    id: operation.id,
                    description: operation.description,
                    status: OperationStatus::Failed,
                    detail,
                });
                break;
            }
        }
    }

    BootstrapK3sReport {
        operations: results,
    }
}

const FAILURE_OUTPUT_CAP: usize = 8192;

fn format_command_failure(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout_trim = stdout.trim();
    let stderr_trim = stderr.trim();

    let mut body = String::new();
    if !stderr_trim.is_empty() {
        body.push_str("stderr: ");
        body.push_str(&cap_utf8(stderr_trim, FAILURE_OUTPUT_CAP / 2));
    }
    if !stdout_trim.is_empty() {
        if !body.is_empty() {
            body.push_str("; ");
        }
        body.push_str("stdout: ");
        body.push_str(&cap_utf8(stdout_trim, FAILURE_OUTPUT_CAP / 2));
    }
    if body.is_empty() {
        body.push_str("(no stdout/stderr captured)");
    }

    format!("exit status {}: {}", output.status, body)
}

fn cap_utf8(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}… ({} more bytes)",
        &text[..end],
        text.len().saturating_sub(end)
    )
}

fn kubectl_nodes_name_output_ok(text: &str) -> bool {
    text.lines().any(|line| line.trim().starts_with("node/"))
}

fn format_dry_run_detail(operation: &K3sPlannedOperation) -> String {
    let args_display = operation.args.join(" ");
    let env_display = redacted_env(&operation.env);
    if env_display.is_empty() {
        format!("dry-run: {} {}", operation.command, args_display)
    } else {
        format!(
            "dry-run: {} {} env=[{}]",
            operation.command, args_display, env_display
        )
    }
}

fn redacted_env(env: &[(String, String)]) -> String {
    env.iter()
        .map(|(key, value)| {
            if is_sensitive_env_key(key) {
                format!("{key}=<redacted>")
            } else {
                format!("{key}={value}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn is_sensitive_env_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    upper == "GITHUB_TOKEN" || upper.contains("SECRET") || upper.contains("TOKEN")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        execute_plan, format_command_failure, format_dry_run_detail, kubectl_nodes_name_output_ok,
    };
    use crate::cli::K3sRole;
    use crate::features::bootstrap::k3s::input::BootstrapK3sConfig;
    use crate::features::bootstrap::k3s::plan::K3sPlannedOperation;
    use crate::runner::{CommandRunner, IoMode};

    struct MockRunner;

    impl CommandRunner for MockRunner {
        fn run_with_env_io(
            &self,
            program: &str,
            args: &[&str],
            _env: &[(&str, &str)],
            _mode: IoMode,
        ) -> anyhow::Result<std::process::Output> {
            if program == "sudo"
                && args.first() == Some(&"k3s")
                && args.get(1) == Some(&"kubectl")
                && args.get(2) == Some(&"get")
            {
                return Ok(std::process::Output {
                    status: std::os::unix::process::ExitStatusExt::from_raw(0),
                    stdout: b"node/cp\n".to_vec(),
                    stderr: vec![],
                });
            }
            Ok(std::process::Output {
                status: std::os::unix::process::ExitStatusExt::from_raw(0),
                stdout: vec![],
                stderr: vec![],
            })
        }
    }

    #[test]
    fn dry_run_detail_redacts_k3s_token_in_env() {
        let op = K3sPlannedOperation {
            id: "run_official_install_script",
            description: "x",
            command: "sh".to_string(),
            args: vec!["/tmp/a.sh".to_string()],
            env: vec![
                ("K3S_URL".to_string(), "https://h:6443".to_string()),
                ("K3S_TOKEN".to_string(), "supersecret".to_string()),
            ],
            failure_is_warning: false,
        };
        let detail = format_dry_run_detail(&op);
        assert!(detail.contains("K3S_TOKEN=<redacted>"));
        assert!(!detail.contains("supersecret"));
        assert!(detail.contains("K3S_URL=https://h:6443"));
    }

    #[test]
    fn kubectl_nodes_output_accepts_node_slash_lines() {
        assert!(kubectl_nodes_name_output_ok("node/foo\n"));
        assert!(!kubectl_nodes_name_output_ok(""));
        assert!(!kubectl_nodes_name_output_ok("NAME\n"));
    }

    #[test]
    fn format_command_failure_includes_stdout_when_stderr_empty() {
        let output = std::process::Output {
            status: std::os::unix::process::ExitStatusExt::from_raw(1),
            stdout: b"k3s wrote this\n".to_vec(),
            stderr: vec![],
        };
        let detail = format_command_failure(&output);
        assert!(detail.contains("stdout: k3s wrote this"));
    }

    #[test]
    fn mock_plan_reaches_kubectl_when_prior_steps_succeed() {
        let config = BootstrapK3sConfig {
            install_script_path: PathBuf::from("/tmp/x.sh"),
            role: K3sRole::Server,
            server_url: None,
            token: None,
            version: None,
            install_exec: None,
            skip_start: false,
            skip_enable: false,
            force: false,
            skip_install: false,
            dry_run: false,
        };
        let plan = vec![
            K3sPlannedOperation {
                id: "download_official_install_script",
                description: "d",
                command: "curl".to_string(),
                args: vec![],
                env: vec![],
                failure_is_warning: false,
            },
            K3sPlannedOperation {
                id: "run_official_install_script",
                description: "i",
                command: "sh".to_string(),
                args: vec![],
                env: vec![],
                failure_is_warning: false,
            },
            K3sPlannedOperation {
                id: "k3s_kubectl_get_nodes",
                description: "v",
                command: "sudo".to_string(),
                args: vec![
                    "k3s".to_string(),
                    "kubectl".to_string(),
                    "get".to_string(),
                    "nodes".to_string(),
                    "-o".to_string(),
                    "name".to_string(),
                ],
                env: vec![],
                failure_is_warning: false,
            },
        ];
        let report = execute_plan(&MockRunner, &config, &plan, IoMode::Buffered);
        assert!(!report.has_failures());
    }
}
