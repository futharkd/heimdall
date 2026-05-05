use anyhow::Result;

use crate::cli::ServiceInfisicalCommand;
use crate::core::operation::{OperationKind, PlannedOperation};
use crate::features::service::{
    ServiceActionKind, ServiceActionPlan, ServiceBackend, ServiceKind, execute_and_print,
    plan_service_actions,
};
use crate::runtime::ExitStatus;

pub fn run(opts: ServiceInfisicalCommand, global: &crate::cli::GlobalOpts) -> Result<ExitStatus> {
    let action: ServiceActionKind = opts.action.into();

    let plan = ServiceActionPlan {
        kind: ServiceKind::Infisical,
        action,
        backend: ServiceBackend::Systemd {
            unit_name: opts.unit_name.clone(),
        },
    };

    let mut ops: Vec<PlannedOperation> = plan_service_actions(plan);
    if matches!(action, ServiceActionKind::Status) {
        ops.push(PlannedOperation {
            id: "service_infisical_cli_status",
            description: "Check infisical CLI on PATH".to_string(),
            kind: OperationKind::Shell {
                command: "command".to_string(),
                args: vec!["-v".to_string(), "infisical".to_string()],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: true,
            verify: None,
        });
    }

    let show_stdout = matches!(action, ServiceActionKind::Status);
    execute_and_print(
        "infisical",
        ops,
        opts.output,
        opts.dry_run,
        global,
        show_stdout,
    )
}
