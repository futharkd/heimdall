use std::io::{self, Write};

use anyhow::{Context, Result, bail};

use crate::cli::{BootstrapUserCommand, OutputFormat};
use crate::modules::user_bootstrap::{BootstrapUserConfig, build_plan, execute_plan};
use crate::output::render_bootstrap_user_human;
use crate::runner::LocalRunner;
use crate::runtime::ExitStatus;

pub fn user(opts: BootstrapUserCommand) -> Result<ExitStatus> {
    let ResolvedInputs { config, output } = resolve_inputs(opts)?;
    let plan = build_plan(&config)?;
    let runner = LocalRunner;
    let report = execute_plan(&runner, &config, &plan);

    match output {
        OutputFormat::Human => println!("{}", render_bootstrap_user_human(&report)),
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
    }

    Ok(if report.has_failures() {
        ExitStatus::Failure
    } else {
        ExitStatus::Success
    })
}

struct ResolvedInputs {
    config: BootstrapUserConfig,
    output: OutputFormat,
}

fn resolve_inputs(opts: BootstrapUserCommand) -> Result<ResolvedInputs> {
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::cli::{BootstrapAction, Cli, Command, OutputFormat};

    #[test]
    fn cli_parses_bootstrap_user_flags() {
        let parsed = Cli::try_parse_from([
            "heimdall",
            "bootstrap",
            "user",
            "--user",
            "admin",
            "--key",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIabc123 admin@host",
            "--output",
            "json",
        ])
        .expect("cli should parse");

        let Command::Bootstrap(bootstrap) = parsed.command else {
            panic!("expected bootstrap command");
        };

        let BootstrapAction::User(user) = bootstrap.action else {
            panic!("expected user action");
        };

        assert_eq!(user.user.as_deref(), Some("admin"));
        assert!(matches!(user.output, OutputFormat::Json));
    }
}
