use crate::core::operation::OperationStatus;
use crate::runner::{CommandRunner, IoMode};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn write_file_with_escalation(
    runner: &dyn CommandRunner,
    path: &Path,
    content: &str,
    mode: Option<u32>,
    io_mode: IoMode,
) -> OperationStatus {
    let path_str = path.display().to_string();
    let needs_sudo = path_str.starts_with("/etc/")
        || path_str.starts_with("/var/")
        || path_str.starts_with("/root/");

    if needs_sudo {
        // Write to temp file, then copy with sudo to preserve permissions
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let temp_path = format!("/tmp/heimdall-write-{}", nanos);

        if fs::write(&temp_path, content).is_err() {
            return OperationStatus::Failed;
        }

        if fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600)).is_err() {
            let _ = fs::remove_file(&temp_path);
            return OperationStatus::Failed;
        }

        // Create parent directory with sudo
        let parent_str = path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "/".to_string());

        let mkdir_args = vec!["mkdir", "-p", &parent_str];
        let mkdir_result = runner.run_with_env_io("sudo", &mkdir_args, &[], io_mode);

        if mkdir_result.is_err()
            || !mkdir_result
                .as_ref()
                .map(|o| o.status.success())
                .unwrap_or(false)
        {
            let _ = fs::remove_file(&temp_path);
            return OperationStatus::Failed;
        }

        // Copy temp file to target with sudo
        let copy_args = vec!["cp", &temp_path, &path_str];

        match runner.run_with_env_io("sudo", &copy_args, &[], io_mode) {
            Ok(output) if output.status.success() => {
                // Set final permissions with sudo if mode was specified
                let final_status = if let Some(m) = mode {
                    let mode_str = format!("{:o}", m);
                    let chmod_args = vec!["chmod", &mode_str, &path_str];
                    match runner.run_with_env_io("sudo", &chmod_args, &[], io_mode) {
                        Ok(output) if output.status.success() => OperationStatus::Succeeded,
                        _ => OperationStatus::Failed,
                    }
                } else {
                    OperationStatus::Succeeded
                };

                let _ = fs::remove_file(&temp_path);
                final_status
            }
            _ => {
                let _ = fs::remove_file(&temp_path);
                OperationStatus::Failed
            }
        }
    } else {
        // Direct write for non-privileged paths
        if let Some(parent) = path.parent()
            && fs::create_dir_all(parent).is_err()
        {
            return OperationStatus::Failed;
        }

        if fs::write(path, content).is_err() {
            return OperationStatus::Failed;
        }

        if let Some(m) = mode
            && fs::set_permissions(path, fs::Permissions::from_mode(m)).is_err()
        {
            return OperationStatus::Failed;
        }

        OperationStatus::Succeeded
    }
}
