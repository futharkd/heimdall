use anyhow::Result;

use crate::cli::ServiceNetbirdCommand;
use crate::features::service::{
    ServiceActionKind, ServiceActionPlan, ServiceBackend, ServiceKind, execute_and_print,
    plan_service_actions,
};
use crate::runtime::ExitStatus;

pub fn run(opts: ServiceNetbirdCommand, global: &crate::cli::GlobalOpts) -> Result<ExitStatus> {
    let action: ServiceActionKind = opts.action.into();
    let plan = ServiceActionPlan {
        kind: ServiceKind::Netbird,
        action,
        backend: ServiceBackend::Systemd {
            unit_name: opts.unit_name,
        },
    };
    let ops = plan_service_actions(plan);
    let show_stdout = matches!(action, ServiceActionKind::Status);
    execute_and_print(
        "netbird",
        ops,
        opts.output,
        opts.dry_run,
        global,
        show_stdout,
    )
}
