use crate::core::elevation::PrivilegeContext;
use crate::core::operation::PlannedOperation;
use crate::features::bootstrap::docker::input::DockerConfig;
use crate::features::bootstrap::docker::report::BootstrapDockerReport;
use crate::runner::{CommandRunner, IoMode, executor::execute_plan as shared_execute};

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &DockerConfig,
    operations: &[PlannedOperation],
    io_mode: IoMode,
) -> BootstrapDockerReport {
    let results = shared_execute(
        operations,
        runner,
        PrivilegeContext::ELEVATED_OPS,
        config.dry_run,
        false,
        io_mode,
    );
    BootstrapDockerReport {
        operations: results,
    }
}
