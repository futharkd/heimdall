use crate::cli::{HardenSshCommand, OutputFormat};
use anyhow::Result;
use inquire::{Confirm, CustomType};

fn map_inquire<T>(r: Result<T, inquire::InquireError>) -> anyhow::Result<T> {
    r.map_err(|e| match e {
        inquire::InquireError::NotTTY => anyhow::anyhow!("not a TTY; pass the flag directly"),
        inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted => {
            anyhow::anyhow!("cancelled")
        }
        other => anyhow::anyhow!("{other}"),
    })
}

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

    let config = HardenSshConfig {
        new_port,
        current_port,
        disable_root_login: opts.disable_root_login,
        disable_password_auth: opts.disable_password_auth,
        dry_run: opts.dry_run,
        assume_yes: opts.yes,
    };

    Ok(ResolvedSshInputs {
        config,
        output: opts.output,
    })
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
