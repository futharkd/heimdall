use crate::core::doctor::DoctorContext;

use super::super::report::{CheckStatus, DoctorCheck};
use super::command_available;

pub fn contribute(ctx: &DoctorContext) -> Vec<DoctorCheck> {
    let on_path = command_available(ctx.runner, ctx.io_mode, "docker");
    vec![DoctorCheck {
        id: "bootstrap_docker",
        description: "Docker bootstrap (docker on PATH)",
        status: if on_path {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        detail: if on_path {
            "docker CLI found on PATH".to_string()
        } else {
            "docker not found on PATH".to_string()
        },
    }]
}
