use anyhow::{Result, bail};
use inquire::Text;

use crate::cli::{OutputFormat, ResetClusterCommand};

const RESET_CONFIRM_TOKEN: &str = "reset-cluster";

fn map_inquire<T>(r: Result<T, inquire::InquireError>) -> anyhow::Result<T> {
    r.map_err(|e| match e {
        inquire::InquireError::NotTTY => anyhow::anyhow!("non-interactive destructive reset requires --confirm {RESET_CONFIRM_TOKEN}"),
        inquire::InquireError::OperationCanceled
        | inquire::InquireError::OperationInterrupted => anyhow::anyhow!("cancelled"),
        other => anyhow::anyhow!("{other}"),
    })
}

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

    let entered = map_inquire(
        Text::new(&format!("Type '{RESET_CONFIRM_TOKEN}' to confirm full k3s+Flux reset:"))
            .prompt(),
    )?;
    if entered.trim() != RESET_CONFIRM_TOKEN {
        bail!("aborted: destructive confirmation token did not match");
    }

    Ok(())
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
