//! Per-area doctor probes. Wire new modules into [`super::registry::build_report`].

mod config;
mod docker;
mod firewall;
mod flux;
mod infisical;
mod k3s;
mod komodo;
mod netbird;
mod ssh;

use crate::core::doctor::DoctorContext;

use super::report::DoctorCheck;

/// Collect all checks by delegating to registered provider modules.
pub fn collect_checks(ctx: &DoctorContext) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    checks.extend(config::contribute());
    checks.extend(k3s::contribute(ctx));
    checks.extend(docker::contribute(ctx));
    checks.extend(flux::contribute(ctx));
    checks.extend(netbird::contribute(ctx));
    checks.extend(infisical::contribute(ctx));
    checks.extend(komodo::contribute());
    checks.extend(ssh::contribute());
    checks.extend(firewall::contribute(ctx));
    checks
}

pub(super) fn command_available(
    runner: &dyn crate::runner::CommandRunner,
    io_mode: crate::runner::IoMode,
    name: &str,
) -> bool {
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'))
    {
        return false;
    }
    let script = format!("command -v {name}");
    runner
        .run_with_env_io("sh", &["-c", &script], &[], io_mode)
        .map(|o| o.status.success())
        .unwrap_or(false)
}
