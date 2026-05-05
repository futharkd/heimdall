use crate::core::doctor::DoctorContext;

use super::super::report::{CheckStatus, DoctorCheck};
use super::command_available;

pub fn contribute(ctx: &DoctorContext) -> Vec<DoctorCheck> {
    let on_path = command_available(ctx.runner, ctx.io_mode, "k3s");
    vec![DoctorCheck {
        id: "bootstrap_k3s",
        description: "k3s bootstrap (binary on PATH)",
        status: if on_path {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        detail: if on_path {
            "k3s executable found on PATH".to_string()
        } else {
            "k3s not found on PATH".to_string()
        },
    }]
}
