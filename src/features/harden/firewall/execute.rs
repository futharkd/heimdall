use crate::core::elevation::PrivilegeContext;
use crate::core::operation::{OperationStatus, PlannedOperation};
use crate::runner::{CommandRunner, IoMode, executor::execute_plan as shared_execute};

use super::input::HardenFirewallConfig;
use super::report::HardenFirewallReport;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &HardenFirewallConfig,
    operations: &[PlannedOperation],
    io_mode: IoMode,
) -> HardenFirewallReport {
    // Convert core OperationResult to OperationResultOwned for report
    let core_results = shared_execute(
        operations,
        runner,
        PrivilegeContext::ELEVATED_OPS,
        config.dry_run,
        false,
        io_mode,
    );
    let results = core_results
        .into_iter()
        .map(|r| super::report::OperationResultOwned {
            id: r.id.to_string(),
            description: r.description,
            status: match r.status {
                OperationStatus::Planned => "planned".to_string(),
                OperationStatus::Skipped => "skipped".to_string(),
                OperationStatus::Succeeded => "succeeded".to_string(),
                OperationStatus::Failed => "failed".to_string(),
            },
            detail: r.detail,
        })
        .collect();

    HardenFirewallReport {
        operations: results,
    }
}
