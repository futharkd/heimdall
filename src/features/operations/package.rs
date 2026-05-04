//! Distro-aware package installation helpers (`dnf`, `microdnf`, `apt-get`).

use crate::runner::sudo::{SudoPolicy, run_with_env_io_sudo};
use crate::runner::{CommandRunner, IoMode};
use anyhow::{Result, anyhow};

/// Supported package managers detected on PATH.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Dnf,
    MicroDnf,
    AptGet,
}

/// Returns the first usable package manager, or `None` if none are available.
pub fn detect_package_manager() -> Option<PackageManager> {
    if command_on_path("dnf") {
        Some(PackageManager::Dnf)
    } else if command_on_path("microdnf") {
        Some(PackageManager::MicroDnf)
    } else if command_on_path("apt-get") {
        Some(PackageManager::AptGet)
    } else {
        None
    }
}

fn command_on_path(cmd: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", &format!("command -v {cmd}")])
        .status()
        .is_ok_and(|s| s.success())
}

/// Builds `(program, argv)` for a non-interactive install of `package`.
pub fn install_invocation(pm: PackageManager, package: &str) -> (&'static str, Vec<String>) {
    match pm {
        PackageManager::Dnf => (
            "dnf",
            vec!["install".to_string(), "-y".to_string(), package.to_string()],
        ),
        PackageManager::MicroDnf => (
            "microdnf",
            vec!["install".to_string(), "-y".to_string(), package.to_string()],
        ),
        PackageManager::AptGet => (
            "apt-get",
            vec![
                "install".to_string(),
                "-y".to_string(),
                "-qq".to_string(),
                package.to_string(),
            ],
        ),
    }
}

/// Idempotent install: `dnf|microdnf|apt-get install -y` under `sudo`.
pub fn run_ensure_package(
    runner: &dyn CommandRunner,
    package: &str,
    io_mode: IoMode,
    op_label: &str,
) -> Result<std::process::Output> {
    let pm = detect_package_manager().ok_or_else(|| {
        anyhow!(
            "no supported package manager on PATH (tried dnf, microdnf, apt-get); \
             install package `{package}` manually and retry"
        )
    })?;
    let (program, args_owned) = install_invocation(pm, package);
    let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
    run_with_env_io_sudo(
        runner,
        program,
        &args,
        &[],
        io_mode,
        SudoPolicy::AlwaysSudo::<fn(&str) -> Result<bool>>,
        op_label,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_invocation_dnf() {
        let (p, a) = install_invocation(PackageManager::Dnf, "foo");
        assert_eq!(p, "dnf");
        assert_eq!(a, ["install", "-y", "foo"]);
    }

    #[test]
    fn install_invocation_apt() {
        let (p, a) = install_invocation(PackageManager::AptGet, "bar");
        assert_eq!(p, "apt-get");
        assert_eq!(a, ["install", "-y", "-qq", "bar"]);
    }
}
