use crate::core::operation::PlannedOperation;
use crate::runner::{CommandRunner, IoMode, executor::execute_plan as shared_execute};

use super::input::BootstrapNetbirdConfig;
use super::report::BootstrapNetbirdReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &BootstrapNetbirdConfig,
    operations: &[PlannedOperation],
    io_mode: IoMode,
) -> BootstrapNetbirdReport {
    let results = shared_execute(operations, runner, config.dry_run, false, io_mode);
    BootstrapNetbirdReport {
        operations: results,
    }
}
