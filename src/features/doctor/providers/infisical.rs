use crate::core::doctor::DoctorContext;

use super::super::report::{CheckStatus, DoctorCheck};
use super::command_available;

pub fn contribute(ctx: &DoctorContext) -> Vec<DoctorCheck> {
    let on_path = command_available(ctx.runner, ctx.io_mode, "infisical");
    vec![DoctorCheck {
        id: "bootstrap_infisical",
        description: "Infisical CLI",
        status: if on_path {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        detail: if on_path {
            "infisical CLI found on PATH".to_string()
        } else {
            "infisical not found on PATH".to_string()
        },
    }]
}
