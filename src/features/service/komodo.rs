use std::path::PathBuf;

use anyhow::Result;

use crate::cli::ServiceKomodoCommand;
use crate::features::service::{
    ServiceActionKind, ServiceActionPlan, ServiceBackend, ServiceKind, execute_and_print,
    plan_service_actions,
};
use crate::runtime::ExitStatus;

#[derive(Debug, Clone, Copy)]
pub enum KomodoMode {
    Core,
    Periphery,
    All,
}

pub fn run(opts: ServiceKomodoCommand, global: &crate::cli::GlobalOpts) -> Result<ExitStatus> {
    let mode = match opts.mode {
        crate::cli::KomodoServiceMode::Core => KomodoMode::Core,
        crate::cli::KomodoServiceMode::Periphery => KomodoMode::Periphery,
        crate::cli::KomodoServiceMode::All => KomodoMode::All,
    };

    let compose_dir = PathBuf::from(
        opts.compose_dir
            .unwrap_or_else(|| "/etc/heimdall/komodo".to_string()),
    );
    let compose_file = compose_dir.join("compose.yaml");
    let project_name = opts.project_name.unwrap_or_else(|| "komodo".to_string());

    let action: ServiceActionKind = opts.action.into();

    let services = match mode {
        KomodoMode::Core => vec!["core".to_string()],
        KomodoMode::Periphery => vec!["periphery".to_string()],
        KomodoMode::All => Vec::new(),
    };

    let plan = ServiceActionPlan {
        kind: ServiceKind::Komodo,
        action,
        backend: ServiceBackend::DockerCompose {
            compose_file,
            env_file: Some(compose_dir.join("compose.env")),
            project_name,
            services,
        },
    };

    let ops = plan_service_actions(plan);
    let show_stdout = matches!(action, ServiceActionKind::Status);
    execute_and_print(
        "komodo",
        ops,
        opts.output,
        opts.dry_run,
        global,
        show_stdout,
    )
}
