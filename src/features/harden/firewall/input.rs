use crate::cli::{HardenFirewallCommand, OutputFormat};
use crate::config;
use crate::runner::read::read_file_with_escalation;
use crate::runner::{IoMode, LocalRunner};
use anyhow::{Result, anyhow};
use inquire::Confirm;

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
pub struct HardenFirewallConfig {
    pub allow_ssh: bool,
    pub allow_established: bool,
    pub allow_http: bool,
    pub allow_https: bool,
    pub custom_rules: Vec<CustomFirewallRule>,
    pub ssh_port: u16,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct CustomFirewallRule {
    pub port: u16,
    pub protocol: String,
}

pub struct ResolvedFirewallInputs {
    pub config: HardenFirewallConfig,
    pub output: OutputFormat,
}

pub fn resolve_inputs(opts: HardenFirewallCommand) -> Result<ResolvedFirewallInputs> {
    // Parse custom rules from strings
    let mut custom_rules = Vec::new();
    for rule_str in opts.custom_rules {
        let rule = parse_custom_rule(&rule_str)?;
        custom_rules.push(rule);
    }

    // Read SSH port from sshd_config or config
    let ssh_port = read_ssh_port()?;

    // Check if any preset flags were explicitly set
    let has_explicit_presets = std::env::args().any(|arg| {
        arg.starts_with("--allow-ssh")
            || arg.starts_with("--allow-established")
            || arg.starts_with("--allow-http")
            || arg.starts_with("--allow-https")
    });

    // If no presets specified, try to prompt; fall back to CLI defaults if not a TTY
    let (allow_ssh, allow_established, allow_http, allow_https) = if !has_explicit_presets {
        match prompt_presets() {
            Ok(t) => t,
            Err(e) if e.to_string().contains("not a TTY") => (
                opts.allow_ssh,
                opts.allow_established,
                opts.allow_http,
                opts.allow_https,
            ),
            Err(e) => return Err(e),
        }
    } else {
        (
            opts.allow_ssh,
            opts.allow_established,
            opts.allow_http,
            opts.allow_https,
        )
    };

    // Check confirmation for risky operation (setting default zone to drop)
    if !opts.yes && !prompt_confirmation()? {
        return Err(anyhow::anyhow!(
            "Firewall hardening requires explicit confirmation"
        ));
    }

    let config = HardenFirewallConfig {
        allow_ssh,
        allow_established,
        allow_http,
        allow_https,
        custom_rules,
        ssh_port,
        dry_run: opts.dry_run,
    };

    Ok(ResolvedFirewallInputs {
        config,
        output: opts.output,
    })
}

fn parse_custom_rule(rule_str: &str) -> Result<CustomFirewallRule> {
    let mut port = None;
    let mut protocol = None;

    for part in rule_str.split(',') {
        let trimmed = part.trim();
        if let Some(p) = trimmed.strip_prefix("port=") {
            port = Some(p.parse::<u16>()?);
        } else if let Some(pr) = trimmed.strip_prefix("protocol=") {
            protocol = Some(pr.to_string());
        }
    }

    let port = port.ok_or_else(|| anyhow!("custom rule missing port: {}", rule_str))?;
    let protocol = protocol.ok_or_else(|| anyhow!("custom rule missing protocol: {}", rule_str))?;

    Ok(CustomFirewallRule { port, protocol })
}

fn read_ssh_port() -> Result<u16> {
    // Try to read from .heimdall config first
    if let Ok((config, _)) = config::load()
        && let Some(harden) = config.harden
        && let Some(ssh) = harden.ssh
        && let Some(port) = ssh.port
    {
        return Ok(port);
    }

    let runner = LocalRunner;
    match read_file_with_escalation(
        &runner,
        std::path::Path::new("/etc/ssh/sshd_config"),
        IoMode::Buffered,
    ) {
        Ok(content) => {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(port_str) = trimmed.strip_prefix("Port ")
                    && let Ok(port) = port_str.trim().parse::<u16>()
                {
                    return Ok(port);
                }
            }
            Ok(22) // default SSH port
        }
        Err(_) => Ok(22), // default if file doesn't exist/can't read
    }
}

fn prompt_presets() -> Result<(bool, bool, bool, bool)> {
    let allow_ssh = map_inquire(
        Confirm::new("Allow SSH access?")
            .with_default(true)
            .prompt(),
    )?;
    let allow_established = map_inquire(
        Confirm::new("Allow established/related connections?")
            .with_default(true)
            .prompt(),
    )?;
    let allow_http = map_inquire(
        Confirm::new("Allow HTTP (port 80)?")
            .with_default(false)
            .prompt(),
    )?;
    let allow_https = map_inquire(
        Confirm::new("Allow HTTPS (port 443)?")
            .with_default(false)
            .prompt(),
    )?;

    Ok((allow_ssh, allow_established, allow_http, allow_https))
}

fn prompt_confirmation() -> Result<bool> {
    map_inquire(
        Confirm::new("Setting default firewall zone to drop (deny all inbound). Continue?")
            .with_default(false)
            .prompt(),
    )
}
