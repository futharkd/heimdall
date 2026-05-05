use crate::core::operation::PlannedOperation;
use crate::runner::{executor::execute_plan as shared_execute, CommandRunner, IoMode};

use super::input::BootstrapUserConfig;
use super::report::BootstrapUserReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &BootstrapUserConfig,
    operations: &[PlannedOperation],
    io_mode: IoMode,
) -> BootstrapUserReport {
    let results = shared_execute(operations, runner, config.dry_run, config.confirmed, io_mode);
    BootstrapUserReport { operations: results }
}
