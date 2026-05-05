use crate::core::operation::PlannedOperation;
use crate::runner::{CommandRunner, IoMode, executor::execute_plan as shared_execute};

use super::input::HardenSshConfig;
use super::report::HardenSshReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &HardenSshConfig,
    operations: &[PlannedOperation],
    io_mode: IoMode,
) -> HardenSshReport {
    // Convert core OperationResult to OperationResultOwned for report
    let core_results = shared_execute(
        operations,
        runner,
        config.dry_run,
        config.assume_yes,
        io_mode,
    );
    let results = core_results
        .into_iter()
        .map(|r| super::report::OperationResultOwned {
            id: r.id.to_string(),
            description: r.description,
            status: match r.status {
                crate::core::operation::OperationStatus::Planned => "planned".to_string(),
                crate::core::operation::OperationStatus::Skipped => "skipped".to_string(),
                crate::core::operation::OperationStatus::Succeeded => "succeeded".to_string(),
                crate::core::operation::OperationStatus::Failed => "failed".to_string(),
            },
            detail: r.detail,
        })
        .collect();

    HardenSshReport {
        operations: results,
    }
}
