use crate::cli::{HardenSshCommand, OutputFormat};
use anyhow::Result;
use inquire::{Confirm, CustomType, MultiSelect};
use std::io::{self, IsTerminal};

fn map_inquire<T>(r: Result<T, inquire::InquireError>) -> anyhow::Result<T> {
    r.map_err(|e| match e {
        inquire::InquireError::NotTTY => anyhow::anyhow!(
            "not a TTY; pass --allow-root-login / --allow-password-auth or use secure defaults"
        ),
        inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted => {
            anyhow::anyhow!("cancelled")
        }
        other => anyhow::anyhow!("{other}"),
    })
}

const OPT_DISABLE_ROOT: &str = "Disable root login (PermitRootLogin no)";
const OPT_DISABLE_PASS: &str = "Disable password authentication (PasswordAuthentication no)";

#[derive(Debug)]
pub struct HardenSshConfig {
    pub new_port: Option<u16>,
    pub current_port: u16,
    pub disable_root_login: bool,
    pub disable_password_auth: bool,
    pub dry_run: bool,
    /// When `true`, skip interactive confirmations (mirrors `--yes` / `SudoOnPermissionDenied` prompt).
    #[allow(dead_code)]
    pub assume_yes: bool,
}

pub struct ResolvedSshInputs {
    pub config: HardenSshConfig,
    pub output: OutputFormat,
}

pub fn resolve_inputs(opts: HardenSshCommand) -> Result<ResolvedSshInputs> {
    let current_port = read_ssh_port()?;
    let new_port = opts.port.or_else(|| prompt_port().ok());

    // Check confirmation for risky operation
    if !opts.yes && !prompt_confirmation()? {
        return Err(anyhow::anyhow!(
            "SSH hardening requires explicit confirmation"
        ));
    }

    let (disable_root_login, disable_password_auth) = resolve_hardening_toggles(&opts)?;

    let config = HardenSshConfig {
        new_port,
        current_port,
        disable_root_login,
        disable_password_auth,
        dry_run: opts.dry_run,
        assume_yes: opts.yes,
    };

    Ok(ResolvedSshInputs {
        config,
        output: opts.output,
    })
}

/// Secure defaults: both disables `true`. CLI flags or interactive MultiSelect override.
fn resolve_hardening_toggles(opts: &HardenSshCommand) -> Result<(bool, bool)> {
    let any_explicit = opts.disable_root_login
        || opts.allow_root_login
        || opts.disable_password_auth
        || opts.allow_password_auth;

    if any_explicit {
        let disable_root_login = if opts.disable_root_login {
            true
        } else {
            !opts.allow_root_login
        };
        let disable_password_auth = if opts.disable_password_auth {
            true
        } else {
            !opts.allow_password_auth
        };
        return Ok((disable_root_login, disable_password_auth));
    }

    let use_multiselect = !opts.yes
        && !opts.dry_run
        && opts.output == OutputFormat::Human
        && io::stdin().is_terminal();

    if use_multiselect {
        let options = vec![OPT_DISABLE_ROOT, OPT_DISABLE_PASS];
        let selected = map_inquire(
            MultiSelect::new("SSH hardening (space toggles, enter confirms):", options)
                .with_all_selected_by_default()
                .prompt(),
        )?;
        let disable_root_login = selected.contains(&OPT_DISABLE_ROOT);
        let disable_password_auth = selected.contains(&OPT_DISABLE_PASS);
        return Ok((disable_root_login, disable_password_auth));
    }

    Ok((true, true))
}

fn read_ssh_port() -> Result<u16> {
    // Try to read from sshd_config
    match std::fs::read_to_string("/etc/ssh/sshd_config") {
        Ok(content) => {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(port_str) = trimmed.strip_prefix("Port ")
                    && let Ok(port) = port_str.trim().parse::<u16>()
                {
                    return Ok(port);
                }
            }
            Ok(22)
        }
        Err(_) => Ok(22),
    }
}

fn prompt_port() -> Result<u16> {
    map_inquire(
        CustomType::<u16>::new("Enter new SSH port:")
            .with_error_message("Please enter a valid port number (1–65535)")
            .prompt(),
    )
}

fn prompt_confirmation() -> Result<bool> {
    map_inquire(
        Confirm::new("This will change SSH configuration. Continue?")
            .with_default(false)
            .prompt(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::OutputFormat;

    fn opts_default() -> HardenSshCommand {
        HardenSshCommand {
            port: None,
            allow_root_login: false,
            allow_password_auth: false,
            disable_root_login: false,
            disable_password_auth: false,
            dry_run: true,
            yes: true,
            output: OutputFormat::Json,
        }
    }

    #[test]
    fn toggles_explicit_legacy_disable() {
        let mut o = opts_default();
        o.disable_root_login = true;
        let (r, p) = resolve_hardening_toggles(&o).unwrap();
        assert!(r);
        assert!(p);
    }

    #[test]
    fn toggles_explicit_allow_root() {
        let mut o = opts_default();
        o.allow_root_login = true;
        let (r, p) = resolve_hardening_toggles(&o).unwrap();
        assert!(!r);
        assert!(p);
    }

    #[test]
    fn toggles_explicit_allow_pass() {
        let mut o = opts_default();
        o.allow_password_auth = true;
        let (r, p) = resolve_hardening_toggles(&o).unwrap();
        assert!(r);
        assert!(!p);
    }

    #[test]
    fn toggles_noninteractive_defaults_secure() {
        let o = opts_default();
        let (r, p) = resolve_hardening_toggles(&o).unwrap();
        assert!(r);
        assert!(p);
    }
}
