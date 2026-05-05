use std::io::{IsTerminal, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::Command;

use anyhow::Result;

use crate::core::elevation::{PrivilegeContext, command_already_elevated};
use crate::core::operation::{
    OperationKind, OperationResult, OperationStatus, PlannedOperation, VerifyStep,
};
use crate::features::operations::run_ensure_package;
use crate::output::Style;
use crate::runner::sudo::{SudoPolicy, run_with_env_io_sudo, run_with_stdin_sudo};

use super::{CommandRunner, IoMode};

pub fn execute_plan_interactive(
    ops: &[PlannedOperation],
    runner: &dyn CommandRunner,
    privilege: PrivilegeContext,
    debug: bool,
    style: &Style,
    show_stdout: bool,
) -> Vec<OperationResult> {
    let mut results = Vec::new();
    let tty = std::io::stdout().is_terminal();

    for op in ops {
        let op_type = match &op.kind {
            OperationKind::Shell { .. } | OperationKind::InheritIo { .. } => "shell",
            OperationKind::WriteFile { .. } => "file",
            OperationKind::EnsurePackage { .. } => "pkg",
        };

        if tty {
            print!("{} {}", style.yellow("[RUN]"), op.description);
            let _ = std::io::stdout().flush();
        } else {
            println!("{} {}", style.yellow("[RUN]"), op.description);
        }

        let exec_result = match &op.kind {
            OperationKind::Shell {
                command,
                args,
                env,
                stdin_input,
            } => execute_shell(
                runner,
                command,
                args,
                env,
                stdin_input.as_deref(),
                privilege,
                &op.description,
                IoMode::Buffered,
            ),
            OperationKind::EnsurePackage { package } => {
                run_ensure_package(runner, package, IoMode::Buffered, &op.description)
            }
            OperationKind::WriteFile {
                path,
                content,
                mode,
            } => execute_write_file(runner, path, content, *mode, IoMode::Buffered),
            OperationKind::InheritIo { command, args } => execute_inherit_io(command, args),
        };

        let status = match &exec_result {
            Ok(output) if output.status.success() => {
                if let Some(verify) = &op.verify {
                    if let Err(_e) =
                        execute_verify(runner, verify, privilege, IoMode::Buffered, &op.description)
                    {
                        OperationStatus::Failed
                    } else {
                        OperationStatus::Succeeded
                    }
                } else {
                    OperationStatus::Succeeded
                }
            }
            Ok(_) => OperationStatus::Failed,
            Err(_) => OperationStatus::Failed,
        };

        let detail = format_operation_detail(&op.kind);
        let indent = if status == OperationStatus::Succeeded {
            "      "
        } else {
            "       "
        };

        if tty {
            print!("\x1b[1A\r\x1b[K");
        }

        let status_marker = match status {
            OperationStatus::Succeeded => style.green("[OK]"),
            OperationStatus::Failed => style.red("[FAIL]"),
            _ => style.yellow("[PLAN]"),
        };

        println!("{} {}", status_marker, op.description);
        println!(
            "{}{} {} {}",
            indent,
            style.dim(op_type),
            style.dim("→"),
            style.dim(&detail)
        );

        if status == OperationStatus::Failed {
            if let Ok(output) = &exec_result {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.trim().is_empty() {
                    for line in stderr.trim().lines() {
                        println!("       {}", line);
                    }
                }
                if debug {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if !stdout.trim().is_empty() {
                        for line in stdout.trim().lines() {
                            println!("       {}", line);
                        }
                    }
                }
            }
        } else if (debug || show_stdout)
            && status == OperationStatus::Succeeded
            && let Ok(output) = &exec_result
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.trim().lines() {
                println!("       {}", line);
            }
        }

        results.push(OperationResult {
            id: op.id,
            description: op.description.clone(),
            status,
            detail: String::new(),
        });

        if status == OperationStatus::Failed && !op.failure_is_warning {
            break;
        }
    }

    results
}

pub fn execute_plan(
    ops: &[PlannedOperation],
    runner: &dyn CommandRunner,
    privilege: PrivilegeContext,
    dry_run: bool,
    yes: bool,
    io_mode: IoMode,
) -> Vec<OperationResult> {
    let mut results = Vec::new();
    let live_ui = !dry_run && matches!(io_mode, IoMode::LiveTee) && std::io::stdout().is_terminal();

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
        if live_ui {
            println!("[RUN] {}", op.description);
        }
        let exec_result = match &op.kind {
            OperationKind::Shell {
                command,
                args,
                env,
                stdin_input,
            } => execute_shell(
                runner,
                command,
                args,
                env,
                stdin_input.as_deref(),
                privilege,
                &op.description,
                // Human mode defaults to hidden child output.
                if live_ui { IoMode::Buffered } else { io_mode },
            ),
            OperationKind::EnsurePackage { package } => run_ensure_package(
                runner,
                package,
                if live_ui { IoMode::Buffered } else { io_mode },
                &op.description,
            ),
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
                    if let Err(_e) =
                        execute_verify(runner, verify, privilege, io_mode, &op.description)
                    {
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

        let mut detail = match &exec_result {
            Ok(output) => {
                if output.status.success() {
                    String::new()
                } else {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let mut rendered = format!("exit: {}", output.status);
                    if !stdout.trim().is_empty() {
                        rendered.push_str("\nstdout:\n");
                        rendered.push_str(stdout.trim_end());
                    }
                    if !stderr.trim().is_empty() {
                        rendered.push_str("\nstderr:\n");
                        rendered.push_str(stderr.trim_end());
                    }
                    rendered
                }
            }
            Err(_e) => "operation failed".to_string(),
        };
        if !op.kind.is_read_only() && op.validation_slot().is_none() {
            if !detail.is_empty() {
                detail.push('\n');
            }
            detail.push_str("warning: operation has no validation step");
        }

        results.push(OperationResult {
            id: op.id,
            description: op.description.clone(),
            status,
            detail,
        });
        if live_ui {
            let marker = match status {
                OperationStatus::Succeeded => "[OK]",
                OperationStatus::Skipped => "[SKIP]",
                OperationStatus::Planned => "[PLAN]",
                OperationStatus::Failed => "[FAIL]",
            };
            println!("{marker} {}", op.description);
            if status == OperationStatus::Failed {
                let failure_detail = results.last().map(|r| r.detail.trim()).unwrap_or("");
                if !failure_detail.is_empty() {
                    println!("{failure_detail}");
                }
            }
        }

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

#[allow(clippy::too_many_arguments)]
fn execute_shell(
    runner: &dyn CommandRunner,
    command: &str,
    args: &[String],
    env: &[(String, String)],
    stdin_input: Option<&str>,
    privilege: PrivilegeContext,
    op_label: &str,
    io_mode: IoMode,
) -> Result<std::process::Output> {
    let policy = shell_policy(privilege, command);
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let env_refs: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    if let Some(stdin) = stdin_input {
        run_with_stdin_sudo(
            runner, command, &args_refs, &env_refs, stdin, io_mode, policy, op_label,
        )
    } else {
        run_with_env_io_sudo(
            runner, command, &args_refs, &env_refs, io_mode, policy, op_label,
        )
    }
}

fn shell_policy(
    privilege: PrivilegeContext,
    command: &str,
) -> SudoPolicy<'_, fn(&str) -> Result<bool>> {
    if privilege.elevate_shell && !command_already_elevated(command) {
        SudoPolicy::AlwaysSudo
    } else {
        SudoPolicy::None
    }
}

fn execute_write_file(
    runner: &dyn CommandRunner,
    path: &Path,
    content: &str,
    mode: Option<u32>,
    io_mode: IoMode,
) -> Result<std::process::Output> {
    use crate::core::operation::OperationStatus;
    use crate::runner::write::write_file_with_escalation;

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

fn execute_verify(
    runner: &dyn CommandRunner,
    verify: &VerifyStep,
    privilege: PrivilegeContext,
    io_mode: IoMode,
    parent_op_label: &str,
) -> Result<()> {
    let args_refs: Vec<&str> = verify.args.iter().map(|s| s.as_str()).collect();
    let label = format!("{parent_op_label} (verify: {})", verify.description);
    let policy = shell_policy(privilege, &verify.command);
    let output = run_with_env_io_sudo(
        runner,
        &verify.command,
        &args_refs,
        &[],
        io_mode,
        policy,
        &label,
    )?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "verify step failed: {}",
            verify.description
        ))
    }
}
