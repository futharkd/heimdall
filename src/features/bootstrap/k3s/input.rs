use std::path::PathBuf;

use anyhow::{Result, bail};
use inquire::Confirm;

use crate::cli::{BootstrapK3sCommand, K3sRole, OutputFormat};
use crate::runner::{CommandRunner, IoMode};

fn map_inquire<T>(r: Result<T, inquire::InquireError>) -> anyhow::Result<T> {
    r.map_err(|e| match e {
        inquire::InquireError::NotTTY => anyhow::anyhow!("not a TTY; pass the flag directly"),
        inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted => {
            anyhow::anyhow!("cancelled")
        }
        other => anyhow::anyhow!("{other}"),
    })
}

#[derive(Debug, Clone)]
pub struct BootstrapK3sConfig {
    pub install_script_path: PathBuf,
    pub role: K3sRole,
    pub server_url: Option<String>,
    pub token: Option<String>,
    pub version: Option<String>,
    pub install_exec: Option<String>,
    pub skip_start: bool,
    pub skip_enable: bool,
    pub force: bool,
    /// When true, plan omits get.k3s.io download and install (set in `command::run` after probe).
    pub skip_install: bool,
    pub dry_run: bool,
}

pub struct ResolvedK3sInputs {
    pub config: BootstrapK3sConfig,
    pub output: OutputFormat,
}

pub fn resolve_inputs(opts: BootstrapK3sCommand) -> Result<ResolvedK3sInputs> {
    let install_script_path = std::env::temp_dir().join(format!(
        "heimdall-k3s-install-{}.sh",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    let version = opts
        .version
        .clone()
        .or_else(|| std::env::var("INSTALL_K3S_VERSION").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let install_exec = opts
        .install_exec
        .clone()
        .or_else(|| std::env::var("INSTALL_K3S_EXEC").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let server_url = if opts.role == K3sRole::Agent {
        opts.server_url
            .clone()
            .or_else(|| std::env::var("K3S_URL").ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    };

    let token = if opts.role == K3sRole::Agent {
        opts.token
            .clone()
            .or_else(|| std::env::var("K3S_TOKEN").ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    };

    if let Some(ref url) = server_url {
        super::validate::validate_k3s_server_url(url)?;
    }

    super::validate::validate_agent_inputs(opts.role, server_url.as_deref(), token.as_deref())?;

    if !(opts.yes || opts.dry_run || confirm_install()?) {
        bail!("aborted: k3s bootstrap was not confirmed");
    }

    if !opts.dry_run {
        eprintln!(
            "note: the official k3s installer typically requires root and will install systemd units and binaries on this host"
        );
    }

    Ok(ResolvedK3sInputs {
        config: BootstrapK3sConfig {
            install_script_path,
            role: opts.role,
            server_url,
            token,
            version,
            install_exec,
            skip_start: opts.skip_start,
            skip_enable: opts.skip_enable,
            force: opts.force,
            skip_install: false,
            dry_run: opts.dry_run,
        },
        output: opts.output,
    })
}

/// Read-only probe: `k3s` on `PATH` (same binary install script provides).
pub fn probe_k3s_on_path(runner: &dyn CommandRunner) -> bool {
    runner
        .run_with_env_io(
            "sh",
            &["-c", "command -v k3s >/dev/null 2>&1"],
            &[],
            IoMode::Buffered,
        )
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn confirm_install() -> Result<bool> {
    map_inquire(
        Confirm::new("This will install or update k3s using the official get.k3s.io install script. Continue?")
            .with_default(false)
            .prompt(),
    )
}

#[cfg(test)]
mod tests {
    use super::probe_k3s_on_path;
    use crate::runner::{CommandRunner, IoMode};

    struct MockProbe(bool);

    impl CommandRunner for MockProbe {
        fn run_with_env_io(
            &self,
            program: &str,
            args: &[&str],
            _env: &[(&str, &str)],
            _mode: IoMode,
        ) -> anyhow::Result<std::process::Output> {
            if program == "sh"
                && args.len() >= 2
                && args[0] == "-c"
                && args[1] == "command -v k3s >/dev/null 2>&1"
            {
                let code = if self.0 { 0 } else { 1 };
                return Ok(std::process::Output {
                    status: std::os::unix::process::ExitStatusExt::from_raw(code),
                    stdout: vec![],
                    stderr: vec![],
                });
            }
            Ok(std::process::Output {
                status: std::os::unix::process::ExitStatusExt::from_raw(1),
                stdout: vec![],
                stderr: vec![],
            })
        }

        fn run_with_stdin(
            &self,
            _program: &str,
            _args: &[&str],
            _env: &[(&str, &str)],
            _stdin_data: &str,
            _mode: IoMode,
        ) -> anyhow::Result<std::process::Output> {
            Ok(std::process::Output {
                status: std::os::unix::process::ExitStatusExt::from_raw(1),
                stdout: vec![],
                stderr: vec![],
            })
        }
    }

    #[test]
    fn probe_k3s_on_path_true_when_sh_probe_succeeds() {
        assert!(probe_k3s_on_path(&MockProbe(true)));
    }

    #[test]
    fn probe_k3s_on_path_false_when_sh_probe_fails() {
        assert!(!probe_k3s_on_path(&MockProbe(false)));
    }
}
