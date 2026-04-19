use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::cli::{BootstrapNetbirdCommand, NetbirdInstallMethod, OutputFormat};

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
    println!();
    println!(
        "NetBird install method (upstream install.sh reads this from the environment Heimdall sets):"
    );
    println!(
        "  [1] Portable — GitHub release tarballs (USE_BIN_INSTALL=true). Fewer distro prompts; good for servers."
    );
    println!(
        "  [2] Package — apt, dnf, or yum as detected on this host (DEBIAN_FRONTEND=noninteractive for apt)."
    );
    loop {
        let answer = prompt("Choice [1/2] (default: 1): ")?;
        let trimmed = answer.trim().to_ascii_lowercase();
        match trimmed.as_str() {
            "" | "1" | "binary" | "b" | "portable" => return Ok(NetbirdInstallMethod::Binary),
            "2" | "package" | "p" | "repo" => return Ok(NetbirdInstallMethod::Package),
            _ => println!("Enter 1 or 2 (or binary / package)."),
        }
    }
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
