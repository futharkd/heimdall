//! Privileged file reads for paths the invoking user cannot open directly.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Result, anyhow};

use crate::core::elevation::path_requires_privilege_escalation;
use crate::runner::{CommandRunner, IoMode};

/// Read `path` as UTF-8, trying `sudo cat` when the direct open fails with permission denied.
///
/// For paths under `/etc/`, `/var/`, or `/root/`, skips the direct read when unprivileged read is
/// unlikely to succeed (optimization — avoids an extra failing syscall).
pub fn read_file_with_escalation(
    runner: &dyn CommandRunner,
    path: &Path,
    io_mode: IoMode,
) -> Result<String> {
    let skip_direct = path_requires_privilege_escalation(path) && unsafe { libc::geteuid() } != 0;

    if !skip_direct {
        match fs::read_to_string(path) {
            Ok(s) => return Ok(s),
            Err(e) if e.kind() == ErrorKind::PermissionDenied => {}
            Err(e) => return Err(anyhow!(e)),
        }
    } else {
        return try_sudo_cat(runner, path, io_mode);
    }

    try_sudo_cat(runner, path, io_mode)
}

fn try_sudo_cat(runner: &dyn CommandRunner, path: &Path, io_mode: IoMode) -> Result<String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow!("path is not valid UTF-8"))?;
    let out = runner.run_with_env_io("sudo", &["-E", "cat", path_str], &[], io_mode)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!(
            "sudo cat {} failed: {}",
            path.display(),
            stderr.trim()
        ));
    }
    String::from_utf8(out.stdout).map_err(|e| anyhow!("{}: invalid UTF-8", e))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};

    use super::*;
    use crate::runner::{CommandRunner, IoMode};

    struct RecordingRunner {
        calls: RefCell<Vec<(String, Vec<String>)>>,
        cat_stdout: &'static [u8],
    }

    impl RecordingRunner {
        fn new(cat_stdout: &'static [u8]) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                cat_stdout,
            }
        }
    }

    impl CommandRunner for RecordingRunner {
        fn run_with_env_io(
            &self,
            program: &str,
            args: &[&str],
            _env: &[(&str, &str)],
            _mode: IoMode,
        ) -> anyhow::Result<Output> {
            self.calls.borrow_mut().push((
                program.to_string(),
                args.iter().map(|s| (*s).to_string()).collect(),
            ));
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: self.cat_stdout.to_vec(),
                stderr: vec![],
            })
        }

        fn run_with_stdin(
            &self,
            program: &str,
            args: &[&str],
            _env: &[(&str, &str)],
            _stdin_data: &str,
            _mode: IoMode,
        ) -> anyhow::Result<Output> {
            self.calls.borrow_mut().push((
                program.to_string(),
                args.iter().map(|s| (*s).to_string()).collect(),
            ));
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: vec![],
                stderr: vec![],
            })
        }
    }

    #[test]
    fn etc_path_skips_direct_open_and_sudo_cats() {
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let runner = RecordingRunner::new(b"probe-ok\n");
        let path = Path::new("/etc/heimdall-read-escalation-probe");
        let text = read_file_with_escalation(&runner, path, IoMode::Buffered).expect("read");
        assert_eq!(text.trim(), "probe-ok");
        let calls = runner.calls.borrow().clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "sudo");
        assert_eq!(
            calls[0].1,
            vec![
                "-E".to_string(),
                "cat".to_string(),
                path.display().to_string()
            ]
        );
    }
}
