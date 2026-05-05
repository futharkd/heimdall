use crate::core::elevation::PrivilegeContext;
use crate::core::operation::PlannedOperation;
use crate::runner::{CommandRunner, IoMode, executor::execute_plan as shared_execute};

use super::input::BootstrapFluxConfig;
use super::report::BootstrapFluxReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &BootstrapFluxConfig,
    operations: &[PlannedOperation],
    io_mode: IoMode,
) -> BootstrapFluxReport {
    let results = shared_execute(
        operations,
        runner,
        PrivilegeContext::ELEVATED_OPS,
        config.dry_run,
        false,
        io_mode,
    );
    BootstrapFluxReport {
        operations: results,
    }
}
