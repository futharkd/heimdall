use super::input::HardenSshConfig;
use super::plan::SshPlannedOperation;
use super::report::{HardenSshReport, OperationResultOwned};
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
                detail: format!("{} {}", op.command, op.args.join(" ")),
            }
        } else {
            match runner.run_with_env_io(
                &op.command,
                &op.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                &[],
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
