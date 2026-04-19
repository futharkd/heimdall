use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli::{OutputFormat, UpdateCommand};

use super::package::{binary_and_checksum_urls, parse_gitlab_repository, validate_package_version};

#[derive(Debug, Clone)]
pub struct UpdateConfig {
    pub dry_run: bool,
    pub yes: bool,
    pub force: bool,
    pub output: OutputFormat,
    pub exe_path: PathBuf,
    pub package_version: String,
    pub binary_url: String,
    pub checksum_url: String,
    pub gitlab_token: Option<String>,
}

pub fn resolve_inputs(opts: UpdateCommand) -> Result<UpdateConfig> {
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        anyhow::bail!("heimdall update is only supported on Linux x86_64");
    }

    let repository = env!("CARGO_PKG_REPOSITORY");
    let (api_origin, encoded_project) = parse_gitlab_repository(repository)
        .with_context(|| format!("invalid CARGO_PKG_REPOSITORY: {repository}"))?;

    let package_version = opts.tag.clone().unwrap_or_else(|| "latest".to_string());
    validate_package_version(&package_version)?;

    let (binary_url, checksum_url) =
        binary_and_checksum_urls(&api_origin, &encoded_project, &package_version);

    let exe_path = std::env::current_exe().context("resolve current executable path")?;

    let gitlab_token = std::env::var("GITLAB_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("PRIVATE_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });

    Ok(UpdateConfig {
        dry_run: opts.dry_run,
        yes: opts.yes,
        force: opts.force,
        output: opts.output,
        exe_path,
        package_version,
        binary_url,
        checksum_url,
        gitlab_token,
    })
}
