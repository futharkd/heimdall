use std::path::PathBuf;

use anyhow::{Result, bail};
use inquire::{Confirm, Select};

use crate::cli::{BootstrapNetbirdCommand, NetbirdInstallMethod, OutputFormat};

fn map_inquire<T>(r: Result<T, inquire::InquireError>) -> anyhow::Result<T> {
    r.map_err(|e| match e {
        inquire::InquireError::NotTTY => anyhow::anyhow!("not a TTY; pass the flag directly"),
        inquire::InquireError::OperationCanceled
        | inquire::InquireError::OperationInterrupted => anyhow::anyhow!("cancelled"),
        other => anyhow::anyhow!("{other}"),
    })
}

#[derive(Debug, Clone)]
pub struct BootstrapNetbirdConfig {
    pub install_script_path: PathBuf,
    pub skip_ui: bool,
    pub release: String,
    pub install_method: NetbirdInstallMethod,
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

    let install_method = resolve_install_method(&opts)?;

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
            install_method,
            github_token,
            setup_key,
            management_url,
            dry_run: opts.dry_run,
        },
        output: opts.output,
    })
}

fn resolve_install_method(opts: &BootstrapNetbirdCommand) -> Result<NetbirdInstallMethod> {
    if let Some(method) = opts.install_method {
        return Ok(method);
    }
    if let Some(method) = parse_install_method_env()? {
        return Ok(method);
    }
    if opts.yes || opts.dry_run {
        return Ok(NetbirdInstallMethod::Binary);
    }
    prompt_install_method()
}

fn parse_install_method_env() -> Result<Option<NetbirdInstallMethod>> {
    let Ok(raw) = std::env::var("HEIMDALL_NETBIRD_INSTALL_METHOD") else {
        return Ok(None);
    };
    let value = raw.trim().to_ascii_lowercase();
    if value.is_empty() {
        return Ok(None);
    }
    match value.as_str() {
        "binary" | "bin" | "1" | "portable" => Ok(Some(NetbirdInstallMethod::Binary)),
        "package" | "repo" | "apt" | "dnf" | "yum" | "2" => Ok(Some(NetbirdInstallMethod::Package)),
        _ => bail!("invalid HEIMDALL_NETBIRD_INSTALL_METHOD={raw:?} (expected binary or package)"),
    }
}

fn prompt_install_method() -> Result<NetbirdInstallMethod> {
    let options = vec![
        "Portable — GitHub release tarballs (fewer distro prompts; good for servers)",
        "Package — apt, dnf, or yum as detected on this host",
    ];
    let choice = map_inquire(
        Select::new("NetBird install method:", options)
            .with_starting_cursor(0)
            .prompt(),
    )?;
    if choice.starts_with("Portable") {
        Ok(NetbirdInstallMethod::Binary)
    } else {
        Ok(NetbirdInstallMethod::Package)
    }
}

fn confirm_install() -> Result<bool> {
    map_inquire(
        Confirm::new("This will install or update NetBird using the official install script. Continue?")
            .with_default(false)
            .prompt(),
    )
}
