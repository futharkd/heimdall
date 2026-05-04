use super::input::HardenSshConfig;
use super::plan::{SshOpKind, SshPlannedOperation};
use super::report::{HardenSshReport, OperationResultOwned};
use crate::features::operations::{detect_package_manager, install_invocation, run_ensure_package};
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
                detail: planned_detail(op),
            }
        } else {
            let attempt_result = match &op.kind {
                SshOpKind::Shell => run_with_env_io_sudo(
                    runner,
                    &op.command,
                    &op.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    &[],
                    io_mode,
                    SudoPolicy::AlwaysSudo::<fn(&str) -> anyhow::Result<bool>>,
                    &op.description,
                ),
                SshOpKind::EnsurePackage { package } => {
                    run_ensure_package(runner, package, io_mode, &op.description)
                }
            };

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

fn planned_detail(op: &SshPlannedOperation) -> String {
    match &op.kind {
        SshOpKind::Shell => {
            format!("sudo {} {}", op.command, op.args.join(" "))
        }
        SshOpKind::EnsurePackage { package } => {
            if let Some(pm) = detect_package_manager() {
                let (prog, argv) = install_invocation(pm, package);
                format!("sudo {} {}", prog, argv.join(" "))
            } else {
                format!("sudo <package-manager> install -y {package}")
            }
        }
    }
}
