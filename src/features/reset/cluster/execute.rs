use crate::core::operation::PlannedOperation;
use crate::runner::{CommandRunner, IoMode, executor::execute_plan as shared_execute};

use super::input::ResetClusterConfig;
use super::report::ResetClusterReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &ResetClusterConfig,
    operations: &[PlannedOperation],
    io_mode: IoMode,
) -> ResetClusterReport {
    let results = shared_execute(operations, runner, config.dry_run, false, io_mode);
    ResetClusterReport {
        operations: results,
    }
}

#[cfg(test)]
mod tests {
    use crate::core::operation::{OperationKind, OperationStatus, PlannedOperation};
    use crate::features::reset::cluster::input::ResetClusterConfig;
    use crate::runner::{CommandRunner, IoMode};
    use std::os::unix::process::ExitStatusExt;

    use super::execute_plan;

    struct MockRunner;

    impl CommandRunner for MockRunner {
        fn run_with_env_io(
            &self,
            _program: &str,
            _args: &[&str],
            _env: &[(&str, &str)],
            _mode: IoMode,
        ) -> anyhow::Result<std::process::Output> {
            Ok(std::process::Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout: vec![],
                stderr: vec![],
            })
        }

        fn run_with_stdin(
            &self,
            _program: &str,
            _args: &[&str],
            _env: &[(&str, &str)],
            _stdin_data: &str,
            _mode: IoMode,
        ) -> anyhow::Result<std::process::Output> {
            Ok(std::process::Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout: vec![],
                stderr: vec![],
            })
        }
    }

    #[test]
    fn dry_run_reports_planned_steps() {
        let cfg = ResetClusterConfig { dry_run: true };
        let plan = vec![PlannedOperation {
            id: "x",
            description: "x".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec!["rm".to_string()],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        }];
        let report = execute_plan(&MockRunner, &cfg, &plan, IoMode::Buffered);
        assert_eq!(report.operations.len(), 1);
        assert_eq!(report.operations[0].status, OperationStatus::Planned);
    }
}
