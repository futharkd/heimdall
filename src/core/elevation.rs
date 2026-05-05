//! Privilege elevation policy for subprocess execution and privileged filesystem paths.
//!
//! Default stance: the Heimdall process stays the invoking user; elevation is **granular**
//! (`sudo` per operation, `sudo cat` for reads) unless optional process re-exec is enabled elsewhere.

use std::path::Path;

/// Controls how [`crate::runner::executor::execute_plan`] runs shell operations and verify steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrivilegeContext {
    /// When true, wrap shell and verify subprocesses with `sudo -E` unless the planned command is already `sudo`.
    pub elevate_shell: bool,
}

impl PrivilegeContext {
    /// Read-only / diagnostic flows that should not wrap arbitrary subprocesses (doctor probes still use [`crate::runner::read::read_file_with_escalation`] where needed).
    pub const USER_SESSION: Self = Self {
        elevate_shell: false,
    };

    /// Bootstrap, harden, reset, and other workflows that mutate system state.
    pub const ELEVATED_OPS: Self = Self {
        elevate_shell: true,
    };
}

impl Default for PrivilegeContext {
    fn default() -> Self {
        Self::USER_SESSION
    }
}

/// Paths that typically require root for read/write (aligned with [`crate::runner::write::write_file_with_escalation`]).
pub fn path_requires_privilege_escalation(path: &Path) -> bool {
    path.to_str().is_some_and(|s| {
        s.starts_with("/etc/") || s.starts_with("/var/") || s.starts_with("/root/")
    })
}

/// Returns true when the planned shell command is already invoked via `sudo` (avoid `sudo sudo …`).
#[must_use]
pub fn command_already_elevated(command: &str) -> bool {
    command == "sudo"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn path_requires_escalation_under_standard_prefixes() {
        assert!(path_requires_privilege_escalation(Path::new("/etc/foo")));
        assert!(path_requires_privilege_escalation(Path::new("/var/lib/x")));
        assert!(path_requires_privilege_escalation(Path::new(
            "/root/.bashrc"
        )));
        assert!(!path_requires_privilege_escalation(Path::new("/tmp/a")));
    }

    #[test]
    fn sudo_command_detected() {
        assert!(command_already_elevated("sudo"));
        assert!(!command_already_elevated("cp"));
    }
}
