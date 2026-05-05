//! Opt-in whole-process re-exec under `sudo -E`. **Default is off** — prefer granular sudo in the executor.
//!
//! Enable with `HEIMDALL_PRIVILEGE_REEXEC=1`. Disable with `HEIMDALL_SKIP_SUDO_REEXEC` or after one re-exec via `HEIMDALL_REEXEC_ACTIVE`.

use anyhow::Result;

/// When env asks for it, replace this process with `sudo -E <exe> <original args…>`.
///
/// Returns `Ok(())` when no re-exec is attempted. On success after [`std::os::unix::process::CommandExt::exec`],
/// this function does not return.
#[cfg(unix)]
pub fn maybe_privilege_reexec() -> Result<()> {
    use std::os::unix::process::CommandExt;

    if std::env::var("HEIMDALL_PRIVILEGE_REEXEC").ok().as_deref() != Some("1") {
        return Ok(());
    }
    if std::env::var("HEIMDALL_SKIP_SUDO_REEXEC").is_ok() {
        return Ok(());
    }
    if std::env::var("HEIMDALL_REEXEC_ACTIVE").is_ok() {
        return Ok(());
    }
    if unsafe { libc::geteuid() } == 0 {
        return Ok(());
    }

    let exe = std::env::current_exe()?;
    let err = std::process::Command::new("sudo")
        .arg("-E")
        .env("HEIMDALL_REEXEC_ACTIVE", "1")
        .arg(&exe)
        .args(std::env::args_os().skip(1))
        .exec();
    Err(anyhow::anyhow!("failed to exec sudo: {err}"))
}

#[cfg(not(unix))]
pub fn maybe_privilege_reexec() -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::maybe_privilege_reexec;

    #[test]
    fn default_does_not_reexec_without_env() {
        assert!(maybe_privilege_reexec().is_ok());
    }
}
