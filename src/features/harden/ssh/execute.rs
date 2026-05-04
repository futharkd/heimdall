use super::input::HardenSshConfig;
use super::plan::SshPlannedOperation;
use super::report::{HardenSshReport, OperationResultOwned};
use crate::runner::sudo::{SudoPolicy, run_with_env_io_sudo};
use crate::runner::{CommandRunner, IoMode};

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &HardenSshConfig,
    operations: &[SshPlannedOperation],
    io_mode: IoMode,
) -> HardenSshReport {
    let mut results = Vec::new();

    for op in operations {
        let result = if config.dry_run {
            OperationResultOwned {
                id: op.id.clone(),
                description: op.description.clone(),
                status: "planned".to_string(),
                detail: format!("sudo {} {}", op.command, op.args.join(" ")),
            }
        } else {
            let attempt_result = run_with_env_io_sudo(
                runner,
                &op.command,
                &op.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                &[],
                io_mode,
                SudoPolicy::AlwaysSudo::<fn(&str) -> anyhow::Result<bool>>,
                &op.description,
            );

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
                        OperationResultOwned {
                            id: op.id.clone(),
                            description: op.description.clone(),
                            status: "failed".to_string(),
                            detail: String::from_utf8_lossy(&output.stderr).to_string(),
                        }
                    }
                }
                Err(e) => OperationResultOwned {
                    id: op.id.clone(),
                    description: op.description.clone(),
                    status: "failed".to_string(),
                    detail: e.to_string(),
                },
            }
        };

        if result.status == "failed" {
            results.push(result);
            break;
        }

        results.push(result);
    }

    HardenSshReport {
        operations: results,
    }
}
