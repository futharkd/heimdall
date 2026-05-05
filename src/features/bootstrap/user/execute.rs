use crate::core::elevation::PrivilegeContext;
use crate::core::operation::PlannedOperation;
use crate::runner::{CommandRunner, IoMode, executor::execute_plan as shared_execute};

use super::input::BootstrapUserConfig;
use super::report::BootstrapUserReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &BootstrapUserConfig,
    operations: &[PlannedOperation],
    io_mode: IoMode,
) -> BootstrapUserReport {
    let results = shared_execute(
        operations,
        runner,
        PrivilegeContext::ELEVATED_OPS,
        config.dry_run,
        config.confirmed,
        io_mode,
    );
    BootstrapUserReport {
        operations: results,
    }
}
