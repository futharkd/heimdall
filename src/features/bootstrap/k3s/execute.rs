use crate::core::operation::PlannedOperation;
use crate::runner::{executor::execute_plan as shared_execute, CommandRunner, IoMode};

use super::input::BootstrapK3sConfig;
use super::report::BootstrapK3sReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &BootstrapK3sConfig,
    operations: &[PlannedOperation],
    io_mode: IoMode,
) -> BootstrapK3sReport {
    let results = shared_execute(operations, runner, config.dry_run, false, io_mode);
    BootstrapK3sReport { operations: results }
}
