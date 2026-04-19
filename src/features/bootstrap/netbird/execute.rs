use crate::core::operation::{OperationResult, OperationStatus};
use crate::runner::CommandRunner;

use super::input::BootstrapNetbirdConfig;
use super::plan::NetbirdPlannedOperation;
use super::report::BootstrapNetbirdReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &BootstrapNetbirdConfig,
    operations: &[NetbirdPlannedOperation],
) -> BootstrapNetbirdReport {
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
            runner.run(&operation.command, &arg_refs)
        } else {
            runner.run_with_env(&operation.command, &arg_refs, &env_refs)
        };

        match outcome {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let mut detail = stdout.trim().to_string();
                if detail.is_empty() {
                    detail = stderr.trim().to_string();
                }
                if operation.id == "netbird_status" && !status_stdout_looks_connected(&detail) {
                    results.push(OperationResult {
                        id: operation.id,
                        description: operation.description,
                        status: OperationStatus::Failed,
                        detail: format!(
                            "netbird status succeeded but output missing connected management/signal lines: {detail}"
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

    BootstrapNetbirdReport {
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

fn status_stdout_looks_connected(text: &str) -> bool {
    text.lines()
        .any(|line| line.trim().starts_with("Management:") && line.contains("Connected"))
        && text
            .lines()
            .any(|line| line.trim().starts_with("Signal:") && line.contains("Connected"))
}

fn format_dry_run_detail(operation: &NetbirdPlannedOperation) -> String {
    let args_display = redacted_args(operation.id, &operation.args).join(" ");
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

fn redacted_args(id: &str, args: &[String]) -> Vec<String> {
    if id != "netbird_up" {
        return args.to_vec();
    }
    let mut out = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--setup-key" && index + 1 < args.len() {
            out.push(args[index].clone());
            out.push("<redacted>".to_string());
            index += 2;
        } else {
            out.push(args[index].clone());
            index += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        execute_plan, format_command_failure, format_dry_run_detail, status_stdout_looks_connected,
    };
    use crate::features::bootstrap::netbird::input::BootstrapNetbirdConfig;
    use crate::features::bootstrap::netbird::plan::NetbirdPlannedOperation;
    use crate::runner::CommandRunner;

    struct MockRunner;

    impl CommandRunner for MockRunner {
        fn run(&self, program: &str, args: &[&str]) -> anyhow::Result<std::process::Output> {
            self.run_with_env(program, args, &[])
        }

        fn run_with_env(
            &self,
            program: &str,
            args: &[&str],
            _env: &[(&str, &str)],
        ) -> anyhow::Result<std::process::Output> {
            if program == "netbird" && args.first() == Some(&"status") {
                return Ok(std::process::Output {
                    status: std::os::unix::process::ExitStatusExt::from_raw(0),
                    stdout: b"Management: Connected\nSignal: Connected\n".to_vec(),
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
    fn dry_run_detail_redacts_github_token_in_env() {
        let op = NetbirdPlannedOperation {
            id: "run_official_install_script",
            description: "x",
            command: "sh".to_string(),
            args: vec!["/tmp/a.sh".to_string()],
            env: vec![
                ("NETBIRD_RELEASE".to_string(), "latest".to_string()),
                ("GITHUB_TOKEN".to_string(), "supersecret".to_string()),
            ],
            failure_is_warning: false,
        };
        let detail = format_dry_run_detail(&op);
        assert!(detail.contains("GITHUB_TOKEN=<redacted>"));
        assert!(!detail.contains("supersecret"));
        assert!(detail.contains("NETBIRD_RELEASE=latest"));
    }

    #[test]
    fn dry_run_detail_redacts_setup_key_in_args() {
        let op = NetbirdPlannedOperation {
            id: "netbird_up",
            description: "x",
            command: "netbird".to_string(),
            args: vec![
                "up".to_string(),
                "--setup-key".to_string(),
                "key-material".to_string(),
            ],
            env: vec![],
            failure_is_warning: false,
        };
        let detail = format_dry_run_detail(&op);
        assert!(detail.contains("<redacted>"));
        assert!(!detail.contains("key-material"));
    }

    #[test]
    fn status_parse_accepts_official_style_lines() {
        let text = "Foo\nManagement: Connected\nSignal: Connected\n";
        assert!(status_stdout_looks_connected(text));
    }

    #[test]
    fn format_command_failure_includes_stdout_when_stderr_empty() {
        let output = std::process::Output {
            status: std::os::unix::process::ExitStatusExt::from_raw(1),
            stdout: b"install script wrote this to stdout\n".to_vec(),
            stderr: vec![],
        };
        let detail = format_command_failure(&output);
        assert!(detail.contains("stdout: install script wrote this to stdout"));
        assert!(!detail.contains("stderr:"));
    }

    #[test]
    fn format_command_failure_joins_stderr_and_stdout() {
        let output = std::process::Output {
            status: std::os::unix::process::ExitStatusExt::from_raw(7),
            stdout: b"out\n".to_vec(),
            stderr: b"err\n".to_vec(),
        };
        let detail = format_command_failure(&output);
        assert!(detail.contains("stderr: err"));
        assert!(detail.contains("stdout: out"));
    }

    #[test]
    fn mock_plan_reaches_status_when_prior_steps_succeed() {
        let config = BootstrapNetbirdConfig {
            install_script_path: PathBuf::from("/tmp/x.sh"),
            skip_ui: true,
            release: "latest".to_string(),
            install_method: crate::cli::NetbirdInstallMethod::Binary,
            github_token: None,
            setup_key: Some("k".to_string()),
            management_url: None,
            dry_run: false,
        };
        let plan = vec![
            NetbirdPlannedOperation {
                id: "download_official_install_script",
                description: "d",
                command: "curl".to_string(),
                args: vec![],
                env: vec![],
                failure_is_warning: false,
            },
            NetbirdPlannedOperation {
                id: "run_official_install_script",
                description: "i",
                command: "sh".to_string(),
                args: vec![],
                env: vec![],
                failure_is_warning: false,
            },
            NetbirdPlannedOperation {
                id: "netbird_up",
                description: "u",
                command: "netbird".to_string(),
                args: vec!["up".to_string()],
                env: vec![],
                failure_is_warning: false,
            },
            NetbirdPlannedOperation {
                id: "netbird_status",
                description: "s",
                command: "netbird".to_string(),
                args: vec!["status".to_string()],
                env: vec![],
                failure_is_warning: false,
            },
        ];
        let report = execute_plan(&MockRunner, &config, &plan);
        assert!(!report.has_failures());
    }
}
