use super::input::HardenFirewallConfig;
use super::plan::FirewallPlannedOperation;
use super::report::{HardenFirewallReport, OperationResultOwned};
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
                detail: format!("{} {}", op.command, op.args.join(" ")),
            }
        } else {
            match runner.run_with_env_io(
                &op.command,
                &op.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                &op.env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect::<Vec<_>>(),
                io_mode,
            ) {
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

        // Stop on failure (unless it's a warning)
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
