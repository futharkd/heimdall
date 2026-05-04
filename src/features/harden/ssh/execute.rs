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
    use inquire::Confirm;

    let mut results = Vec::new();
    let mut sudo_approved = false;

    for op in operations {
        let result = if config.dry_run {
            OperationResultOwned {
                id: op.id.clone(),
                description: op.description.clone(),
                status: "planned".to_string(),
                detail: format!("{} {}", op.command, op.args.join(" ")),
            }
        } else {
            let mut attempt_result = runner.run_with_env_io(
                &op.command,
                &op.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                &[],
                io_mode,
            );

            #[allow(clippy::collapsible_if)]
            if let Ok(output) = &attempt_result {
                if !output.status.success() && is_permission_error(&output.stderr) {
                    let should_retry = if sudo_approved {
                        true
                    } else {
                        let prompt = format!(
                            "Operation '{}' requires elevated privileges. Retry with sudo?",
                            op.description
                        );
                        match Confirm::new(&prompt).prompt() {
                            Ok(true) => {
                                sudo_approved = true;
                                true
                            }
                            _ => false,
                        }
                    };

                    if should_retry {
                        let mut sudo_args = vec![op.command.as_str()];
                        sudo_args.extend(op.args.iter().map(|s| s.as_str()));

                        attempt_result = runner.run_with_env_io("sudo", &sudo_args, &[], io_mode);
                    }
                }
            }

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

fn is_permission_error(stderr: &[u8]) -> bool {
    let stderr_str = String::from_utf8_lossy(stderr);
    stderr_str.contains("Permission denied")
        || stderr_str.contains("EACCES")
        || stderr_str.contains("Operation not permitted")
        || stderr_str.contains("Access denied")
}
