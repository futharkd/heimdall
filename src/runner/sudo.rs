use std::process::Output;

use anyhow::Result;

use super::{CommandRunner, IoMode};

/// Returns `true` when `stderr` indicates an OS-level permission denial.
///
/// This is intentionally conservative: it only matches well-known POSIX/Linux
/// strings. If a command fails for a different reason the caller should not
/// escalate to `sudo`.
pub fn permission_denied_in_stderr(stderr: &[u8]) -> bool {
    let s = String::from_utf8_lossy(stderr);
    s.contains("Permission denied")
        || s.contains("EACCES")
        || s.contains("Operation not permitted")
        || s.contains("Access denied")
}

/// Controls how a command is invoked with respect to privilege elevation.
///
/// # Avoiding double `sudo`
///
/// If a plan already sets `command = "sudo"` you must use `SudoPolicy::None`;
/// wrapping again would produce `sudo sudo …`.
#[allow(dead_code)]
pub enum SudoPolicy<'a, F>
where
    F: FnMut(&str) -> Result<bool>,
{
    /// Run the command once, without any elevation (default behaviour).
    None,

    /// Prefix every invocation with `sudo` — no fallback, no prompt.
    ///
    /// Suitable for features that are known to require root (e.g. `harden ssh`).
    AlwaysSudo,

    /// Try unprivileged first. On a permission-denied failure, call `prompt`
    /// with a description of the operation. If the user approves (or
    /// `*elevated` is already `true` from a prior op), retry with `sudo`.
    ///
    /// `elevated` is a caller-owned flag that persists across multiple
    /// `run_with_env_io_sudo` calls in a loop: once the user approves once,
    /// subsequent ops skip the prompt.
    ///
    /// # `sudo` env-var stripping
    ///
    /// `sudo` may drop env vars passed via `env` unless `sudo` is configured
    /// with `SETENV` or the caller adds `--preserve-env`. Pass only env vars
    /// that survive your system's `sudo` policy, or use `AlwaysSudo` with
    /// `env = &[]` and set vars via the command itself.
    SudoOnPermissionDenied { elevated: &'a mut bool, prompt: F },
}

/// Run `program args` on `runner` according to `policy`, returning the
/// final [`Output`].
///
/// The `op_label` string is forwarded to the `SudoOnPermissionDenied` prompt
/// callback and ignored for other policies.
pub fn run_with_env_io_sudo<F>(
    runner: &dyn CommandRunner,
    program: &str,
    args: &[&str],
    env: &[(&str, &str)],
    mode: IoMode,
    policy: SudoPolicy<'_, F>,
    op_label: &str,
) -> Result<Output>
where
    F: FnMut(&str) -> Result<bool>,
{
    match policy {
        SudoPolicy::None => runner.run_with_env_io(program, args, env, mode),

        SudoPolicy::AlwaysSudo => {
            // Preserve the invoking user's environment for the elevated child (e.g. `KUBECONFIG`).
            let mut sudo_args = vec!["-E", program];
            sudo_args.extend_from_slice(args);
            runner.run_with_env_io("sudo", &sudo_args, env, mode)
        }

        SudoPolicy::SudoOnPermissionDenied {
            elevated,
            mut prompt,
        } => {
            let output = runner.run_with_env_io(program, args, env, mode)?;

            if output.status.success() || !permission_denied_in_stderr(&output.stderr) {
                return Ok(output);
            }

            let should_retry = if *elevated {
                true
            } else {
                let approved = prompt(op_label)?;
                if approved {
                    *elevated = true;
                }
                approved
            };

            if should_retry {
                let mut sudo_args = vec!["-E", program];
                sudo_args.extend_from_slice(args);
                runner.run_with_env_io("sudo", &sudo_args, env, mode)
            } else {
                Ok(output)
            }
        }
    }
}

/// Like [`run_with_env_io_sudo`] but feeds `stdin_data` to the child (used by shell ops with stdin).
#[allow(clippy::too_many_arguments)]
pub fn run_with_stdin_sudo<F>(
    runner: &dyn CommandRunner,
    program: &str,
    args: &[&str],
    env: &[(&str, &str)],
    stdin_data: &str,
    mode: IoMode,
    policy: SudoPolicy<'_, F>,
    op_label: &str,
) -> Result<Output>
where
    F: FnMut(&str) -> Result<bool>,
{
    match policy {
        SudoPolicy::None => runner.run_with_stdin(program, args, env, stdin_data, mode),

        SudoPolicy::AlwaysSudo => {
            let mut sudo_args: Vec<&str> = Vec::with_capacity(2 + args.len());
            sudo_args.push("-E");
            sudo_args.push(program);
            sudo_args.extend_from_slice(args);
            runner.run_with_stdin("sudo", &sudo_args, env, stdin_data, mode)
        }

        SudoPolicy::SudoOnPermissionDenied {
            elevated,
            mut prompt,
        } => {
            let output = runner.run_with_stdin(program, args, env, stdin_data, mode)?;

            if output.status.success() || !permission_denied_in_stderr(&output.stderr) {
                return Ok(output);
            }

            let should_retry = if *elevated {
                true
            } else {
                let approved = prompt(op_label)?;
                if approved {
                    *elevated = true;
                }
                approved
            };

            if should_retry {
                let mut sudo_args: Vec<&str> = Vec::with_capacity(2 + args.len());
                sudo_args.push("-E");
                sudo_args.push(program);
                sudo_args.extend_from_slice(args);
                runner.run_with_stdin("sudo", &sudo_args, env, stdin_data, mode)
            } else {
                Ok(output)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    use super::*;

    // ── permission_denied_in_stderr ──────────────────────────────────────────

    #[test]
    fn detects_permission_denied() {
        assert!(permission_denied_in_stderr(
            b"cp: /etc/foo: Permission denied"
        ));
    }

    #[test]
    fn detects_eacces() {
        assert!(permission_denied_in_stderr(b"EACCES"));
    }

    #[test]
    fn detects_operation_not_permitted() {
        assert!(permission_denied_in_stderr(b"Operation not permitted"));
    }

    #[test]
    fn detects_access_denied() {
        assert!(permission_denied_in_stderr(b"Access denied"));
    }

    #[test]
    fn no_false_positive_on_other_error() {
        assert!(!permission_denied_in_stderr(b"No such file or directory"));
        assert!(!permission_denied_in_stderr(b""));
    }

    // ── test double ─────────────────────────────────────────────────────────

    /// Records every invocation as `(program, args)` and returns a preset
    /// sequence of `Output` values.
    struct ScriptedRunner {
        calls: RefCell<Vec<(String, Vec<String>)>>,
        responses: RefCell<Vec<Output>>,
    }

    fn make_output(exit_code: i32, stderr: &[u8]) -> Output {
        Output {
            status: ExitStatus::from_raw(exit_code << 8),
            stdout: vec![],
            stderr: stderr.to_vec(),
        }
    }

    impl ScriptedRunner {
        fn new(responses: Vec<Output>) -> Self {
            Self {
                calls: RefCell::new(vec![]),
                responses: RefCell::new(responses),
            }
        }

        fn calls(&self) -> Vec<(String, Vec<String>)> {
            self.calls.borrow().clone()
        }
    }

    impl CommandRunner for ScriptedRunner {
        fn run_with_env_io(
            &self,
            program: &str,
            args: &[&str],
            _env: &[(&str, &str)],
            _mode: IoMode,
        ) -> Result<Output> {
            self.calls.borrow_mut().push((
                program.to_string(),
                args.iter().map(|s| s.to_string()).collect(),
            ));

            let mut responses = self.responses.borrow_mut();
            if responses.is_empty() {
                Ok(make_output(0, b""))
            } else {
                Ok(responses.remove(0))
            }
        }

        fn run_with_stdin(
            &self,
            program: &str,
            args: &[&str],
            _env: &[(&str, &str)],
            _stdin_data: &str,
            _mode: IoMode,
        ) -> Result<Output> {
            self.calls.borrow_mut().push((
                program.to_string(),
                args.iter().map(|s| s.to_string()).collect(),
            ));
            Ok(make_output(0, b""))
        }
    }

    // ── SudoPolicy::None ────────────────────────────────────────────────────

    #[test]
    fn none_calls_program_directly() {
        let runner = ScriptedRunner::new(vec![make_output(0, b"")]);
        let out = run_with_env_io_sudo(
            &runner,
            "echo",
            &["hello"],
            &[],
            IoMode::Buffered,
            SudoPolicy::None::<fn(&str) -> Result<bool>>,
            "test op",
        )
        .unwrap();

        assert!(out.status.success());
        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "echo");
        assert_eq!(calls[0].1, ["hello"]);
    }

    // ── SudoPolicy::AlwaysSudo ───────────────────────────────────────────────

    #[test]
    fn always_sudo_wraps_program_in_single_call() {
        let runner = ScriptedRunner::new(vec![make_output(0, b"")]);
        run_with_env_io_sudo(
            &runner,
            "cp",
            &["/a", "/b"],
            &[],
            IoMode::Buffered,
            SudoPolicy::AlwaysSudo::<fn(&str) -> Result<bool>>,
            "copy config",
        )
        .unwrap();

        let calls = runner.calls();
        assert_eq!(calls.len(), 1, "should issue exactly one call");
        assert_eq!(calls[0].0, "sudo");
        assert_eq!(calls[0].1, ["-E", "cp", "/a", "/b"]);
    }

    // ── SudoPolicy::SudoOnPermissionDenied ───────────────────────────────────

    #[test]
    fn fallback_retries_with_sudo_on_permission_denied() {
        let runner = ScriptedRunner::new(vec![
            make_output(1, b"cp: /etc/foo: Permission denied"),
            make_output(0, b""),
        ]);

        let mut elevated = false;
        let mut prompt_calls = 0usize;
        let out = run_with_env_io_sudo(
            &runner,
            "cp",
            &["/a", "/b"],
            &[],
            IoMode::Buffered,
            SudoPolicy::SudoOnPermissionDenied {
                elevated: &mut elevated,
                prompt: |_label| {
                    prompt_calls += 1;
                    Ok(true)
                },
            },
            "copy config",
        )
        .unwrap();

        assert!(out.status.success());
        assert!(elevated, "elevated flag must be set after approval");
        assert_eq!(prompt_calls, 1);

        let calls = runner.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0], ("cp".into(), vec!["/a".into(), "/b".into()]));
        assert_eq!(
            calls[1],
            (
                "sudo".into(),
                vec!["-E".into(), "cp".into(), "/a".into(), "/b".into()]
            )
        );
    }

    #[test]
    fn fallback_skips_prompt_when_already_elevated() {
        let runner = ScriptedRunner::new(vec![
            make_output(1, b"Permission denied"),
            make_output(0, b""),
        ]);

        let mut elevated = true; // already approved from a prior op
        let mut prompt_calls = 0usize;
        run_with_env_io_sudo(
            &runner,
            "systemctl",
            &["reload", "sshd"],
            &[],
            IoMode::Buffered,
            SudoPolicy::SudoOnPermissionDenied {
                elevated: &mut elevated,
                prompt: |_| {
                    prompt_calls += 1;
                    Ok(true)
                },
            },
            "reload sshd",
        )
        .unwrap();

        assert_eq!(prompt_calls, 0, "must not prompt when already elevated");
        let calls = runner.calls();
        assert_eq!(calls[1].0, "sudo");
        assert_eq!(
            calls[1].1.first().map(|s| s.as_str()),
            Some("-E"),
            "sudo retry must preserve env"
        );
    }

    #[test]
    fn fallback_returns_original_output_when_user_declines() {
        let denied_output = make_output(1, b"Permission denied");
        let runner = ScriptedRunner::new(vec![denied_output]);

        let mut elevated = false;
        let out = run_with_env_io_sudo(
            &runner,
            "cp",
            &["/a", "/b"],
            &[],
            IoMode::Buffered,
            SudoPolicy::SudoOnPermissionDenied {
                elevated: &mut elevated,
                prompt: |_| Ok(false),
            },
            "copy config",
        )
        .unwrap();

        assert!(!out.status.success());
        assert!(!elevated);
        assert_eq!(runner.calls().len(), 1, "no retry when user declines");
    }

    #[test]
    fn fallback_does_not_retry_on_non_permission_error() {
        let runner = ScriptedRunner::new(vec![make_output(1, b"No such file or directory")]);

        let mut elevated = false;
        let mut prompt_calls = 0usize;
        let out = run_with_env_io_sudo(
            &runner,
            "cp",
            &["/a", "/b"],
            &[],
            IoMode::Buffered,
            SudoPolicy::SudoOnPermissionDenied {
                elevated: &mut elevated,
                prompt: |_| {
                    prompt_calls += 1;
                    Ok(true)
                },
            },
            "copy config",
        )
        .unwrap();

        assert!(!out.status.success());
        assert_eq!(prompt_calls, 0, "must not prompt on non-permission error");
        assert_eq!(runner.calls().len(), 1);
    }
}
