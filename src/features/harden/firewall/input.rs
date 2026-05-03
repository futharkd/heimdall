use crate::cli::{HardenFirewallCommand, OutputFormat};
use crate::config;
use anyhow::{anyhow, Result};
use std::io::{self, IsTerminal, Write};

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

    // If no presets specified and TTY available, prompt for each
    let (allow_ssh, allow_established, allow_http, allow_https) =
        if !has_explicit_presets && io::stdin().is_terminal() {
            prompt_presets()?
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
        return Err(anyhow::anyhow!("Firewall hardening requires explicit confirmation"));
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
    if let Ok((config, _)) = config::load() {
        if let Some(harden) = config.harden {
            if let Some(ssh) = harden.ssh {
                if let Some(port) = ssh.port {
                    return Ok(port);
                }
            }
        }
    }

    // Read from sshd_config
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
            Ok(22) // default SSH port
        }
        Err(_) => Ok(22), // default if file doesn't exist/can't read
    }
}

fn prompt_presets() -> Result<(bool, bool, bool, bool)> {
    let allow_ssh = confirm("Allow SSH access? [yes/no]: ")?;
    let allow_established = confirm("Allow established/related connections? [yes/no]: ")?;
    let allow_http = confirm("Allow HTTP (port 80)? [yes/no]: ")?;
    let allow_https = confirm("Allow HTTPS (port 443)? [yes/no]: ")?;

    Ok((allow_ssh, allow_established, allow_http, allow_https))
}

fn prompt_confirmation() -> Result<bool> {
    confirm(
        "Setting default firewall zone to drop (deny all inbound). Continue? [yes/no]: ",
    )
}

fn confirm(label: &str) -> Result<bool> {
    print!("{}", label);
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_lowercase() == "yes")
}
