use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::cli::{BootstrapNetbirdCommand, OutputFormat};

#[derive(Debug, Clone)]
pub struct BootstrapNetbirdConfig {
    pub install_script_path: PathBuf,
    pub skip_ui: bool,
    pub release: String,
    pub github_token: Option<String>,
    pub setup_key: Option<String>,
    pub management_url: Option<String>,
    pub dry_run: bool,
}

pub struct ResolvedNetbirdInputs {
    pub config: BootstrapNetbirdConfig,
    pub output: OutputFormat,
}

pub fn resolve_inputs(opts: BootstrapNetbirdCommand) -> Result<ResolvedNetbirdInputs> {
    let install_script_path = std::env::temp_dir().join(format!(
        "heimdall-netbird-install-{}.sh",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    let release = opts
        .release
        .clone()
        .or_else(|| std::env::var("NETBIRD_RELEASE").ok())
        .unwrap_or_else(|| "latest".to_string());

    let setup_key = opts
        .setup_key
        .clone()
        .or_else(|| std::env::var("NETBIRD_SETUP_KEY").ok());

    let management_url = opts
        .management_url
        .clone()
        .or_else(|| std::env::var("NETBIRD_MANAGEMENT_URL").ok());

    if let Some(ref url) = management_url {
        super::validate::validate_management_url(url)?;
    }

    let github_token = std::env::var("GITHUB_TOKEN").ok();

    if !(opts.yes || opts.dry_run || confirm_install()?) {
        bail!("aborted: NetBird bootstrap was not confirmed");
    }

    if !opts.dry_run && setup_key.is_none() {
        eprintln!(
            "note: no setup key provided; `netbird up` will use interactive SSO if a desktop session is available"
        );
    }

    Ok(ResolvedNetbirdInputs {
        config: BootstrapNetbirdConfig {
            install_script_path,
            skip_ui: opts.skip_ui,
            release,
            github_token,
            setup_key,
            management_url,
            dry_run: opts.dry_run,
        },
        output: opts.output,
    })
}

fn confirm_install() -> Result<bool> {
    let answer = prompt(
        "This will install or update NetBird using the official install script. Continue? type 'yes' to proceed: ",
    )?;
    Ok(answer == "yes")
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}
