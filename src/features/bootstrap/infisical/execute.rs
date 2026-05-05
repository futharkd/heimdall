use crate::core::operation::{OperationResult, OperationStatus};
use crate::features::bootstrap::infisical::plan::InfisicalPlannedOperation;
use crate::features::bootstrap::infisical::report::BootstrapInfisicalReport;
use crate::runner::{CommandRunner, IoMode};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    operations: Vec<InfisicalPlannedOperation>,
    io_mode: IoMode,
) -> BootstrapInfisicalReport {
    let mut results = vec![];

    for op in operations {
        match op {
            InfisicalPlannedOperation::Subprocess {
                id,
                description,
                command,
                args,
                env,
                failure_is_warning,
            } => {
                let status = if matches!(io_mode, IoMode::Buffered) {
                    OperationStatus::Planned
                } else {
                    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                    let env_refs: Vec<(&str, &str)> =
                        env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

                    match runner.run_with_env_io(
                        &command,
                        args_refs.as_slice(),
                        env_refs.as_slice(),
                        io_mode,
                    ) {
                        Ok(output) => {
                            if output.status.success() {
                                OperationStatus::Succeeded
                            } else {
                                OperationStatus::Failed
                            }
                        }
                        Err(_) => {
                            if failure_is_warning {
                                OperationStatus::Skipped
                            } else {
                                OperationStatus::Failed
                            }
                        }
                    }
                };

                results.push(OperationResult {
                    id,
                    description: description.to_string(),
                    status,
                    detail: String::new(),
                });

                if status == OperationStatus::Failed {
                    break;
                }
            }
            InfisicalPlannedOperation::InheritIo {
                id,
                description,
                command,
                args,
            } => {
                let status = if matches!(io_mode, IoMode::Buffered) {
                    OperationStatus::Planned
                } else {
                    let mut cmd = Command::new(&command);
                    cmd.args(&args);

                    match cmd.status() {
                        Ok(status) if status.success() => OperationStatus::Succeeded,
                        _ => OperationStatus::Failed,
                    }
                };

                results.push(OperationResult {
                    id,
                    description: description.to_string(),
                    status,
                    detail: String::new(),
                });

                if status == OperationStatus::Failed {
                    break;
                }
            }
            InfisicalPlannedOperation::WriteFile {
                id,
                description,
                path,
                content,
                mode,
            } => {
                let status = if matches!(io_mode, IoMode::Buffered) {
                    OperationStatus::Planned
                } else {
                    if let Some(parent) = path.parent() {
                        if fs::create_dir_all(parent).is_err() {
                            OperationStatus::Failed
                        } else {
                            match fs::write(&path, &content) {
                                Ok(_) => {
                                    if fs::set_permissions(&path, fs::Permissions::from_mode(mode))
                                        .is_err()
                                    {
                                        OperationStatus::Failed
                                    } else {
                                        OperationStatus::Succeeded
                                    }
                                }
                                Err(_) => OperationStatus::Failed,
                            }
                        }
                    } else {
                        OperationStatus::Failed
                    }
                };

                results.push(OperationResult {
                    id,
                    description: description.to_string(),
                    status,
                    detail: path.display().to_string(),
                });

                if status == OperationStatus::Failed {
                    break;
                }
            }
        }
    }

    BootstrapInfisicalReport {
        operations: results,
    }
}
