use crate::core::operation::{OperationResult, OperationStatus};
use crate::features::bootstrap::komodo::plan::KomodoPlannedOperation;
use crate::features::bootstrap::komodo::report::BootstrapKomodoReport;
use crate::runner::{CommandRunner, IoMode};
use std::fs;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    _config: &crate::features::bootstrap::komodo::input::BootstrapKomodoConfig,
    operations: Vec<KomodoPlannedOperation>,
    io_mode: IoMode,
) -> BootstrapKomodoReport {
    let mut results = vec![];

    for op in operations {
        match op {
            KomodoPlannedOperation::Subprocess {
                id,
                description,
                command,
                args,
                env,
                failure_is_warning,
            } => {
                let status = if matches!(io_mode, IoMode::Buffered) {
                    // Dry-run: just report as planned
                    OperationStatus::Planned
                } else {
                    // Real execution - convert Vec to slice references
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
                    description,
                    status,
                    detail: String::new(),
                });

                // Stop on failure (unless warning)
                if status == OperationStatus::Failed {
                    break;
                }
            }
            KomodoPlannedOperation::WriteFile {
                id,
                description,
                path,
                content,
            } => {
                let status = if matches!(io_mode, IoMode::Buffered) {
                    // Dry-run
                    OperationStatus::Planned
                } else {
                    // Actually write the file
                    if let Some(parent) = path.parent() {
                        if let Err(_) = fs::create_dir_all(parent) {
                            OperationStatus::Failed
                        } else {
                            match fs::write(&path, &content) {
                                Ok(_) => OperationStatus::Succeeded,
                                Err(_) => OperationStatus::Failed,
                            }
                        }
                    } else {
                        OperationStatus::Failed
                    }
                };

                results.push(OperationResult {
                    id,
                    description,
                    status,
                    detail: path.display().to_string(),
                });

                // Stop on failure
                if status == OperationStatus::Failed {
                    break;
                }
            }
        }
    }

    BootstrapKomodoReport {
        operations: results,
    }
}
