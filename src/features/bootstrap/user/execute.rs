use crate::core::operation::{OperationResult, OperationStatus, PlannedOperation};
use crate::runner::CommandRunner;

use super::input::BootstrapUserConfig;
use super::report::BootstrapUserReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &BootstrapUserConfig,
    operations: &[PlannedOperation],
) -> BootstrapUserReport {
    let mut results = Vec::with_capacity(operations.len());

    for operation in operations {
        if operation.requires_confirmation && !config.confirmed {
            results.push(OperationResult {
                id: operation.id,
                description: operation.description,
                status: OperationStatus::Skipped,
                detail: "skipped because risky changes were not confirmed".to_string(),
            });
            continue;
        }

        if config.dry_run {
            results.push(OperationResult {
                id: operation.id,
                description: operation.description,
                status: OperationStatus::Planned,
                detail: format!(
                    "dry-run: {} {}",
                    operation.command,
                    operation.args.join(" ")
                ),
            });
            continue;
        }

        let arg_refs: Vec<&str> = operation.args.iter().map(String::as_str).collect();
        match runner.run(&operation.command, &arg_refs) {
            Ok(output) if output.status.success() => results.push(OperationResult {
                id: operation.id,
                description: operation.description,
                status: OperationStatus::Succeeded,
                detail: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            }),
            Ok(output) => {
                results.push(OperationResult {
                    id: operation.id,
                    description: operation.description,
                    status: OperationStatus::Failed,
                    detail: format!(
                        "exit status {}: {}",
                        output.status,
                        String::from_utf8_lossy(&output.stderr).trim()
                    ),
                });
                break;
            }
            Err(err) => {
                results.push(OperationResult {
                    id: operation.id,
                    description: operation.description,
                    status: OperationStatus::Failed,
                    detail: format!("failed to execute: {err}"),
                });
                break;
            }
        }
    }

    BootstrapUserReport {
        operations: results,
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::execute_plan;
    use crate::features::bootstrap::user::input::BootstrapUserConfig;
    use crate::features::bootstrap::user::plan::build_plan;
    use crate::runner::CommandRunner;

    struct MockRunner {
        fail_after: Option<usize>,
        calls: std::sync::Mutex<usize>,
    }

    impl CommandRunner for MockRunner {
        fn run(&self, _program: &str, _args: &[&str]) -> Result<std::process::Output> {
            let mut guard = self.calls.lock().expect("lock");
            *guard += 1;
            if self.fail_after.is_some_and(|n| *guard >= n) {
                return Ok(std::process::Output {
                    status: std::os::unix::process::ExitStatusExt::from_raw(1 << 8),
                    stdout: vec![],
                    stderr: b"boom".to_vec(),
                });
            }
            Ok(std::process::Output {
                status: std::os::unix::process::ExitStatusExt::from_raw(0),
                stdout: b"ok".to_vec(),
                stderr: vec![],
            })
        }
    }

    fn config() -> BootstrapUserConfig {
        BootstrapUserConfig {
            user: "admin".to_string(),
            group: "admin".to_string(),
            keys: vec!["ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIabc123 user@host".to_string()],
            disable_root_login: true,
            disable_password_auth: false,
            dry_run: false,
            confirmed: true,
        }
    }

    #[test]
    fn execute_stops_on_failure() {
        let c = config();
        let plan = build_plan(&c).expect("plan should build");
        let runner = MockRunner {
            fail_after: Some(2),
            calls: std::sync::Mutex::new(0),
        };
        let report = execute_plan(&runner, &c, &plan);
        assert!(report.has_failures());
    }

    #[test]
    fn risky_steps_skipped_without_confirmation() {
        let mut c = config();
        c.confirmed = false;
        let plan = build_plan(&c).expect("plan should build");
        let runner = MockRunner {
            fail_after: None,
            calls: std::sync::Mutex::new(0),
        };
        let report = execute_plan(&runner, &c, &plan);
        assert!(
            report
                .operations
                .iter()
                .any(|op| op.status == crate::core::operation::OperationStatus::Skipped)
        );
    }
}
