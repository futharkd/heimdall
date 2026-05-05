use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::Command;

use anyhow::Result;

use crate::core::operation::{
    OperationKind, OperationResult, OperationStatus, PlannedOperation, VerifyStep,
};
use crate::features::operations::run_ensure_package;

use super::{CommandRunner, IoMode};

pub fn execute_plan(
    ops: &[PlannedOperation],
    runner: &dyn CommandRunner,
    dry_run: bool,
    yes: bool,
    io_mode: IoMode,
) -> Vec<OperationResult> {
    let mut results = Vec::new();

    for op in ops {
        // Dry-run: emit planned op with formatted detail
        if dry_run {
            let detail = format_operation_detail(&op.kind);
            let detail = if let Some(verify) = &op.verify {
                format!("{}\n  → verify: {}", detail, verify.description)
            } else {
                detail
            };
            results.push(OperationResult {
                id: op.id,
                description: op.description.clone(),
                status: OperationStatus::Planned,
                detail,
            });
            continue;
        }

        // Confirmation gate
        if op.requires_confirmation
            && !yes
            && let Ok(false) = inquire::Confirm::new(&format!("{}?", op.description)).prompt()
        {
            results.push(OperationResult {
                id: op.id,
                description: op.description.clone(),
                status: OperationStatus::Skipped,
                detail: "user declined".to_string(),
            });
            continue;
        }

        // Execute primary operation
        let exec_result = match &op.kind {
            OperationKind::Shell {
                command,
                args,
                env,
                stdin_input,
            } => execute_shell(runner, command, args, env, stdin_input.as_deref(), io_mode),
            OperationKind::EnsurePackage { package } => {
                run_ensure_package(runner, package, io_mode, &op.description)
            }
            OperationKind::WriteFile {
                path,
                content,
                mode,
            } => execute_write_file(runner, path, content, *mode, io_mode),
            OperationKind::InheritIo { command, args } => execute_inherit_io(command, args),
        };

        // Process result
        let status = match &exec_result {
            Ok(output) if output.status.success() => {
                // Primary op succeeded; run verify if present
                if let Some(verify) = &op.verify {
                    if let Err(_e) = execute_verify(runner, verify, io_mode) {
                        OperationStatus::Failed
                    } else {
                        OperationStatus::Succeeded
                    }
                } else {
                    OperationStatus::Succeeded
                }
            }
            Ok(_) => {
                // Primary op failed (non-zero exit)
                OperationStatus::Failed
            }
            Err(_) => {
                // Primary op errored (couldn't spawn, etc.)
                OperationStatus::Failed
            }
        };

        let detail = match &exec_result {
            Ok(output) => String::from_utf8_lossy(&output.stderr).to_string(),
            Err(_e) => "operation failed".to_string(),
        };

        results.push(OperationResult {
            id: op.id,
            description: op.description.clone(),
            status,
            detail,
        });

        // Break on failure unless failure_is_warning
        if status == OperationStatus::Failed && !op.failure_is_warning {
            break;
        }
    }

    results
}

fn format_operation_detail(kind: &OperationKind) -> String {
    match kind {
        OperationKind::Shell {
            command,
            args,
            env: _,
            stdin_input: _,
        } => {
            let mut s = format!("{} {}", command, args.join(" "));
            if s.is_empty() {
                s = "(no command)".to_string();
            }
            s
        }
        OperationKind::EnsurePackage { package } => {
            format!("ensure package: {}", package)
        }
        OperationKind::WriteFile {
            path,
            content: _,
            mode,
        } => {
            let mut s = format!("write {}", path.display());
            if let Some(m) = mode {
                s.push_str(&format!(" (mode: {:o})", m));
            }
            s
        }
        OperationKind::InheritIo { command, args } => {
            format!("{} {} (interactive)", command, args.join(" "))
        }
    }
}

fn execute_shell(
    runner: &dyn CommandRunner,
    command: &str,
    args: &[String],
    env: &[(String, String)],
    stdin_input: Option<&str>,
    io_mode: IoMode,
) -> Result<std::process::Output> {
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let env_refs: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    if let Some(stdin) = stdin_input {
        runner.run_with_stdin(command, &args_refs, &env_refs, stdin, io_mode)
    } else {
        runner.run_with_env_io(command, &args_refs, &env_refs, io_mode)
    }
}

fn execute_write_file(
    runner: &dyn CommandRunner,
    path: &Path,
    content: &str,
    mode: Option<u32>,
    io_mode: IoMode,
) -> Result<std::process::Output> {
    use crate::runner::write::write_file_with_escalation;
    use crate::core::operation::OperationStatus;

    let status = write_file_with_escalation(runner, path, content, mode, io_mode);

    // Convert OperationStatus to Result<Output>
    match status {
        OperationStatus::Succeeded => Ok(std::process::Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        }),
        _ => Err(anyhow::anyhow!("failed to write file")),
    }
}

fn execute_inherit_io(command: &str, args: &[String]) -> Result<std::process::Output> {
    let status = Command::new(command).args(args).status()?;
    Ok(std::process::Output {
        status,
        stdout: Vec::new(),
        stderr: Vec::new(),
    })
}

fn execute_verify(runner: &dyn CommandRunner, verify: &VerifyStep, io_mode: IoMode) -> Result<()> {
    let args_refs: Vec<&str> = verify.args.iter().map(|s| s.as_str()).collect();
    let output = runner.run_with_env_io(&verify.command, &args_refs, &[], io_mode)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "verify step failed: {}",
            verify.description
        ))
    }
}
