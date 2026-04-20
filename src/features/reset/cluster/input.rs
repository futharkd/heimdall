use std::io::{self, IsTerminal, Write};

use anyhow::{Result, bail};

use crate::cli::{OutputFormat, ResetClusterCommand};

const RESET_CONFIRM_TOKEN: &str = "reset-cluster";

#[derive(Debug, Clone)]
pub struct ResetClusterConfig {
    pub dry_run: bool,
}

pub struct ResolvedResetClusterInputs {
    pub config: ResetClusterConfig,
    pub output: OutputFormat,
}

pub fn resolve_inputs(opts: ResetClusterCommand) -> Result<ResolvedResetClusterInputs> {
    ensure_destructive_confirmed(&opts)?;
    Ok(ResolvedResetClusterInputs {
        config: ResetClusterConfig {
            dry_run: opts.dry_run,
        },
        output: opts.output,
    })
}

fn ensure_destructive_confirmed(opts: &ResetClusterCommand) -> Result<()> {
    if opts.dry_run {
        return Ok(());
    }

    if !opts.yes {
        bail!("refusing destructive reset without --yes (or use --dry-run to preview)");
    }

    if let Some(token) = opts.confirm.as_deref().map(str::trim)
        && token == RESET_CONFIRM_TOKEN
    {
        return Ok(());
    }

    if !io::stdin().is_terminal() {
        bail!(
            "non-interactive destructive reset requires --confirm {RESET_CONFIRM_TOKEN}"
        );
    }

    let entered = prompt(&format!(
        "Type '{RESET_CONFIRM_TOKEN}' to confirm full k3s+Flux reset: "
    ))?;
    if entered.trim() == RESET_CONFIRM_TOKEN {
        return Ok(());
    }

    bail!("aborted: destructive confirmation token did not match");
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::ensure_destructive_confirmed;
    use crate::cli::{OutputFormat, ResetClusterCommand};

    fn cmd() -> ResetClusterCommand {
        ResetClusterCommand {
            dry_run: false,
            yes: false,
            confirm: None,
            output: OutputFormat::Human,
        }
    }

    #[test]
    fn dry_run_skips_confirmation() {
        let mut c = cmd();
        c.dry_run = true;
        assert!(ensure_destructive_confirmed(&c).is_ok());
    }

    #[test]
    fn destructive_requires_yes() {
        let err = ensure_destructive_confirmed(&cmd()).expect_err("must fail");
        assert!(err.to_string().contains("--yes"));
    }

    #[test]
    fn destructive_accepts_matching_confirm_flag() {
        let mut c = cmd();
        c.yes = true;
        c.confirm = Some("reset-cluster".to_string());
        assert!(ensure_destructive_confirmed(&c).is_ok());
    }
}
