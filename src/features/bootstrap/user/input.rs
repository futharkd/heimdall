use std::io::{self, Write};

use anyhow::{Context, Result, bail};

use crate::cli::{BootstrapUserCommand, OutputFormat};

#[derive(Debug, Clone)]
pub struct BootstrapUserConfig {
    pub user: String,
    pub group: String,
    pub keys: Vec<String>,
    pub disable_root_login: bool,
    pub disable_password_auth: bool,
    pub dry_run: bool,
    pub confirmed: bool,
}

pub struct ResolvedInputs {
    pub config: BootstrapUserConfig,
    pub output: OutputFormat,
}

pub fn resolve_inputs(opts: BootstrapUserCommand) -> Result<ResolvedInputs> {
    let user = match opts.user {
        Some(value) => value,
        None => prompt("Enter admin username: ")?,
    };
    if user.trim().is_empty() {
        bail!("username cannot be empty");
    }

    let group = opts.group.unwrap_or_else(|| user.clone());
    let mut keys = opts.keys;
    for key_file in opts.key_files {
        let content = std::fs::read_to_string(&key_file)
            .with_context(|| format!("failed to read key file: {key_file}"))?;
        keys.extend(
            content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToOwned::to_owned),
        );
    }

    if keys.is_empty() {
        println!("No SSH keys provided.");
        println!("Paste one or more allowed SSH public keys, then submit an empty line:");
        loop {
            let line = prompt("> ")?;
            if line.trim().is_empty() {
                break;
            }
            keys.push(line);
        }
    }

    let risky = opts.disable_root_login || opts.disable_password_auth;
    let confirmed = if opts.yes || !risky {
        true
    } else {
        confirm_risky_changes()?
    };

    Ok(ResolvedInputs {
        config: BootstrapUserConfig {
            user,
            group,
            keys,
            disable_root_login: opts.disable_root_login,
            disable_password_auth: opts.disable_password_auth,
            dry_run: opts.dry_run,
            confirmed,
        },
        output: opts.output,
    })
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

fn confirm_risky_changes() -> Result<bool> {
    let answer =
        prompt("Risky SSH authentication changes requested. Continue? type 'yes' to proceed: ")?;
    Ok(answer == "yes")
}
