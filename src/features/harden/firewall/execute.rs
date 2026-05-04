use super::input::HardenFirewallConfig;
use super::plan::{FirewallOpKind, FirewallPlannedOperation};
use super::report::{HardenFirewallReport, OperationResultOwned};
use crate::features::operations::{detect_package_manager, install_invocation, run_ensure_package};
use crate::runner::sudo::{SudoPolicy, run_with_env_io_sudo};
use crate::runner::{CommandRunner, IoMode};

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &HardenFirewallConfig,
    operations: &[FirewallPlannedOperation],
    io_mode: IoMode,
) -> HardenFirewallReport {
    let mut results = Vec::new();

    for op in operations {
        let result = if config.dry_run {
            OperationResultOwned {
                id: op.id.clone(),
                description: op.description.clone(),
                status: "planned".to_string(),
                detail: planned_detail(op),
            }
        } else {
            let env_refs: Vec<(&str, &str)> = op
                .env
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();

            let attempt_result = match &op.kind {
                FirewallOpKind::Shell => run_with_env_io_sudo(
                    runner,
                    &op.command,
                    &op.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    &env_refs,
                    io_mode,
                    SudoPolicy::AlwaysSudo::<fn(&str) -> anyhow::Result<bool>>,
                    &op.description,
                ),
                FirewallOpKind::EnsurePackage { package } => {
                    run_ensure_package(runner, package, io_mode, &op.description)
                }
            };

            match attempt_result {
                Ok(output) => {
                    if output.status.success() {
                        OperationResultOwned {
                            id: op.id.clone(),
                            description: op.description.clone(),
                            status: "succeeded".to_string(),
                            detail: String::from_utf8_lossy(&output.stdout).to_string(),
                        }
                    } else {
                        let status = if op.failure_is_warning {
                            "skipped".to_string()
                        } else {
                            "failed".to_string()
                        };
                        OperationResultOwned {
                            id: op.id.clone(),
                            description: op.description.clone(),
                            status,
                            detail: String::from_utf8_lossy(&output.stderr).to_string(),
                        }
                    }
                }
                Err(e) => {
                    let status = if op.failure_is_warning {
                        "skipped".to_string()
                    } else {
                        "failed".to_string()
                    };
                    OperationResultOwned {
                        id: op.id.clone(),
                        description: op.description.clone(),
                        status,
                        detail: e.to_string(),
                    }
                }
            }
        };

        if result.status == "failed" {
            results.push(result);
            break;
        }

        results.push(result);
    }

    HardenFirewallReport {
        operations: results,
    }
}

fn planned_detail(op: &FirewallPlannedOperation) -> String {
    match &op.kind {
        FirewallOpKind::Shell => {
            format!("sudo {} {}", op.command, op.args.join(" "))
        }
        FirewallOpKind::EnsurePackage { package } => {
            if let Some(pm) = detect_package_manager() {
                let (prog, argv) = install_invocation(pm, package);
                format!("sudo {} {}", prog, argv.join(" "))
            } else {
                format!("sudo <package-manager> install -y {package}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_dry_run_yields_planned() {
        let config = HardenFirewallConfig {
            allow_ssh: true,
            allow_established: true,
            allow_http: false,
            allow_https: false,
            custom_rules: vec![],
            ssh_port: 22,
            dry_run: true,
        };

        let ops = vec![FirewallPlannedOperation {
            id: "test_op".to_string(),
            description: "Test operation".to_string(),
            kind: FirewallOpKind::Shell,
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            env: vec![],
            failure_is_warning: false,
        }];

        let runner = MockRunner;
        let report = execute_plan(&runner, &config, &ops, IoMode::Buffered);

        assert_eq!(report.operations.len(), 1);
        assert_eq!(report.operations[0].status, "planned");
    }
}
