use anyhow::{Context, Result, bail};
use inquire::{Confirm, Password, Text};

use crate::cli::{BootstrapUserCommand, OutputFormat};
use crate::runner::read::read_file_with_escalation;
use crate::runner::{IoMode, LocalRunner};

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
pub struct BootstrapUserConfig {
    pub user: String,
    pub group: String,
    pub keys: Vec<String>,
    pub password: String,
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
        None => {
            let s = map_inquire(Text::new("Enter admin username:").prompt())?;
            if s.trim().is_empty() {
                bail!("username cannot be empty");
            }
            s
        }
    };

    let group = opts.group.unwrap_or_else(|| user.clone());
    let mut keys = opts.keys;
    let runner = LocalRunner;
    for key_file in opts.key_files {
        let content =
            read_file_with_escalation(&runner, std::path::Path::new(&key_file), IoMode::Buffered)
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
            let line = map_inquire(
                Text::new(">")
                    .with_help_message("leave empty to finish")
                    .prompt(),
            )?;
            if line.trim().is_empty() {
                break;
            }
            keys.push(line);
        }
    }

    let password = match opts.password {
        Some(p) => p,
        None => map_inquire(Password::new("Password for the new user (used for sudo):").prompt())?,
    };

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
            password,
            disable_root_login: opts.disable_root_login,
            disable_password_auth: opts.disable_password_auth,
            dry_run: opts.dry_run,
            confirmed,
        },
        output: opts.output,
    })
}

fn confirm_risky_changes() -> Result<bool> {
    map_inquire(
        Confirm::new("Risky SSH authentication changes requested. Continue?")
            .with_default(false)
            .prompt(),
    )
}
