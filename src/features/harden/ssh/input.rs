use crate::cli::{HardenSshCommand, OutputFormat};
use anyhow::Result;
use std::io::{self, IsTerminal, Write};

#[derive(Debug)]
pub struct HardenSshConfig {
    pub new_port: Option<u16>,
    pub current_port: u16,
    pub disable_root_login: bool,
    pub disable_password_auth: bool,
    pub dry_run: bool,
}

pub struct ResolvedSshInputs {
    pub config: HardenSshConfig,
    pub output: OutputFormat,
}

pub fn resolve_inputs(opts: HardenSshCommand) -> Result<ResolvedSshInputs> {
    let current_port = read_ssh_port()?;
    let new_port = opts.port.or_else(|| {
        if io::stdin().is_terminal() {
            prompt_port().ok()
        } else {
            None
        }
    });

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
                if trimmed.starts_with("Port ") {
                    if let Ok(port) = trimmed[5..].trim().parse::<u16>() {
                        return Ok(port);
                    }
                }
            }
            Ok(22)
        }
        Err(_) => Ok(22),
    }
}

fn prompt_port() -> Result<u16> {
    print!("Enter new SSH port: ");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().parse::<u16>()?)
}

fn prompt_confirmation() -> Result<bool> {
    print!("This will change SSH configuration. Continue? [yes/no]: ");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_lowercase() == "yes")
}
