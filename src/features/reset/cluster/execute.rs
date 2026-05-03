use crate::core::operation::{OperationResult, OperationStatus};
use crate::runner::{CommandRunner, IoMode};

use super::input::ResetClusterConfig;
use super::plan::ResetPlannedOperation;
use super::report::ResetClusterReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &ResetClusterConfig,
    operations: &[ResetPlannedOperation],
    io_mode: IoMode,
) -> ResetClusterReport {
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

    ResetClusterReport {
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

fn format_dry_run_detail(operation: &ResetPlannedOperation) -> String {
    let args_display = operation.args.join(" ");
    if operation.env.is_empty() {
        format!("dry-run: {} {}", operation.command, args_display)
    } else {
        let env_display = operation
            .env
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "dry-run: {} {} env=[{}]",
            operation.command, args_display, env_display
        )
    }
}

#[cfg(test)]
mod tests {
    use super::execute_plan;
    use crate::core::operation::OperationStatus;
    use crate::features::reset::cluster::input::ResetClusterConfig;
    use crate::features::reset::cluster::plan::ResetPlannedOperation;
    use crate::runner::{CommandRunner, IoMode};

    struct MockRunner;

    impl CommandRunner for MockRunner {
        fn run_with_env_io(
            &self,
            _program: &str,
            _args: &[&str],
            _env: &[(&str, &str)],
            _mode: IoMode,
        ) -> anyhow::Result<std::process::Output> {
            Ok(std::process::Output {
                status: std::os::unix::process::ExitStatusExt::from_raw(0),
                stdout: vec![],
                stderr: vec![],
            })
        }

        fn run_with_stdin(
            &self,
            _program: &str,
            _args: &[&str],
            _env: &[(&str, &str)],
            _stdin_data: &str,
            _mode: IoMode,
        ) -> anyhow::Result<std::process::Output> {
            Ok(std::process::Output {
                status: std::os::unix::process::ExitStatusExt::from_raw(0),
                stdout: vec![],
                stderr: vec![],
            })
        }
    }

    #[test]
    fn dry_run_reports_planned_steps() {
        let cfg = ResetClusterConfig { dry_run: true };
        let plan = vec![ResetPlannedOperation {
            id: "x",
            description: "x",
            command: "sudo".to_string(),
            args: vec!["rm".to_string()],
            env: vec![],
            failure_is_warning: false,
        }];
        let report = execute_plan(&MockRunner, &cfg, &plan, IoMode::Buffered);
        assert_eq!(report.operations.len(), 1);
        assert_eq!(report.operations[0].status, OperationStatus::Planned);
    }
}
