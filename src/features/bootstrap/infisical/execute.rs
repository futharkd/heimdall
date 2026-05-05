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
                    let path_str = path.display().to_string();
                    let needs_sudo = path_str.starts_with("/etc/")
                        || path_str.starts_with("/var/")
                        || path_str.starts_with("/root/");

                    if needs_sudo {
                        // Write to temp file, then copy with sudo to preserve permissions
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let nanos = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_nanos())
                            .unwrap_or(0);
                        let temp_path = format!("/tmp/infisical-{}", nanos);

                        if fs::write(&temp_path, &content).is_err() {
                            OperationStatus::Failed
                        } else if fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600))
                            .is_err()
                        {
                            let _ = fs::remove_file(&temp_path);
                            OperationStatus::Failed
                        } else {
                            // Copy temp file to target with sudo
                            let copy_args = vec!["cp", &temp_path, &path_str];

                            match runner.run_with_env_io("sudo", &copy_args, &[], io_mode) {
                                Ok(output) if output.status.success() => {
                                    // Set final permissions with sudo
                                    let mode_str = format!("{:o}", mode);
                                    let chmod_args = vec!["chmod", &mode_str, &path_str];
                                    let chmod_status =
                                        runner.run_with_env_io("sudo", &chmod_args, &[], io_mode);
                                    let _ = fs::remove_file(&temp_path);

                                    match chmod_status {
                                        Ok(output) if output.status.success() => {
                                            OperationStatus::Succeeded
                                        }
                                        _ => OperationStatus::Failed,
                                    }
                                }
                                _ => {
                                    let _ = fs::remove_file(&temp_path);
                                    OperationStatus::Failed
                                }
                            }
                        }
                    } else {
                        if let Some(parent) = path.parent() {
                            if fs::create_dir_all(parent).is_err() {
                                OperationStatus::Failed
                            } else {
                                match fs::write(&path, &content) {
                                    Ok(_) => {
                                        if fs::set_permissions(
                                            &path,
                                            fs::Permissions::from_mode(mode),
                                        )
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
