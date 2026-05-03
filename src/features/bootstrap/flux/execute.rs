use crate::core::operation::{OperationResult, OperationStatus};
use crate::runner::{CommandRunner, IoMode};

use super::input::BootstrapFluxConfig;
use super::plan::FluxPlannedOperation;
use super::report::BootstrapFluxReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &BootstrapFluxConfig,
    operations: &[FluxPlannedOperation],
    io_mode: IoMode,
) -> BootstrapFluxReport {
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

    BootstrapFluxReport {
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

fn format_dry_run_detail(operation: &FluxPlannedOperation) -> String {
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
    if id != "flux_bootstrap_git" {
        return args.to_vec();
    }
    let mut out = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg.starts_with("--private-key-file=") {
            out.push("--private-key-file=<redacted>".to_string());
            index += 1;
            continue;
        }
        if arg == "--password" && index + 1 < args.len() {
            out.push("--password".to_string());
            out.push("<redacted>".to_string());
            index += 2;
            continue;
        }
        out.push(arg.clone());
        index += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{execute_plan, format_dry_run_detail};
    use crate::features::bootstrap::flux::input::BootstrapFluxConfig;
    use crate::features::bootstrap::flux::plan::FluxPlannedOperation;
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
    fn dry_run_redacts_private_key_file_and_password() {
        let op = FluxPlannedOperation {
            id: "flux_bootstrap_git",
            description: "x",
            command: "flux".to_string(),
            args: vec![
                "bootstrap".to_string(),
                "git".to_string(),
                "--url=ssh://git@x/y.git".to_string(),
                "--private-key-file=/secret/key".to_string(),
                "--password".to_string(),
                "hunter2".to_string(),
            ],
            env: vec![],
            failure_is_warning: false,
        };
        let d = format_dry_run_detail(&op);
        assert!(d.contains("--private-key-file=<redacted>"));
        assert!(d.contains("<redacted>"));
        assert!(!d.contains("hunter2"));
        assert!(!d.contains("/secret/key"));
    }

    #[test]
    fn mock_reconcile_plan_completes() {
        let cfg = BootstrapFluxConfig {
            install_script_path: PathBuf::from("/tmp/x.sh"),
            install_script_url: "https://fluxcd.io/install.sh".to_string(),
            git_url: "ssh://git@x/y.git".to_string(),
            branch: "main".to_string(),
            cluster_path: "c".to_string(),
            namespace: "flux-system".to_string(),
            kubeconfig: "/k".to_string(),
            dry_run: false,
            force: false,
            skip_flux_cli_install: false,
            namespace_exists: true,
            byok_private_key: None,
            private_key_bootstrap_path: None,
            ephemeral_key_pair_root: None,
            ephemeral_key_generated: false,
            private_key_passphrase: None,
            keep_generated_key_dir: None,
            kube_elevated: false,
        };
        let plan = vec![
            FluxPlannedOperation {
                id: "flux_reconcile_source_git",
                description: "a",
                command: "flux".to_string(),
                args: vec![],
                env: vec![],
                failure_is_warning: false,
            },
            FluxPlannedOperation {
                id: "flux_reconcile_kustomization",
                description: "b",
                command: "flux".to_string(),
                args: vec![],
                env: vec![],
                failure_is_warning: false,
            },
            FluxPlannedOperation {
                id: "flux_get_kustomization",
                description: "c",
                command: "flux".to_string(),
                args: vec![],
                env: vec![],
                failure_is_warning: false,
            },
        ];
        let report = execute_plan(&MockRunner, &cfg, &plan, IoMode::Buffered);
        assert!(!report.has_failures());
    }
}
