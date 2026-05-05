use crate::cli::{BootstrapInfisicalCommand, OutputFormat};
use crate::config::InfisicalState;
use crate::features::bootstrap::infisical::validate;
use anyhow::Result;
use inquire::{Select, Text};
use std::io::IsTerminal;

#[derive(Debug, Clone)]
pub struct BootstrapInfisicalConfig {
    pub address: String,
    pub project_slug: String,
    pub project_id: String,
    pub environment: String,
    pub node_name: String,
    pub folders: Vec<String>,
    pub client_id: String,
    pub client_secret: String,
    pub secrets_dir: String,
    pub config_dir: String,
    pub skip_install: bool,
    pub dry_run: bool,
    pub output: OutputFormat,
}

pub struct ResolvedInfisicalInputs {
    pub config: BootstrapInfisicalConfig,
}

fn map_inquire<T>(r: Result<T, inquire::InquireError>) -> anyhow::Result<T> {
    r.map_err(|e| match e {
        inquire::InquireError::NotTTY => anyhow::anyhow!("not a TTY; pass the flag directly"),
        inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted => {
            anyhow::anyhow!("cancelled")
        }
        other => anyhow::anyhow!("{other}"),
    })
}

fn resolve_project_slug(
    opts: &BootstrapInfisicalCommand,
    saved: &InfisicalState,
) -> Result<String> {
    if let Some(slug) = &opts.project_slug {
        return Ok(slug.clone());
    }

    if !std::io::stdin().is_terminal() {
        if let Some(saved_slug) = &saved.project_slug {
            return Ok(saved_slug.clone());
        }
        return Err(anyhow::anyhow!(
            "project_slug is required; pass --project-slug or INFISICAL_PROJECT_SLUG"
        ));
    }

    let mut prompt = Text::new("Infisical project slug:");
    if let Some(s) = saved.project_slug.as_deref() {
        prompt = prompt.with_default(s);
    }
    let slug = map_inquire(prompt.prompt())?;
    let trimmed = slug.trim();
    if trimmed.is_empty() {
        Err(anyhow::anyhow!("project slug cannot be empty"))
    } else {
        Ok(trimmed.to_string())
    }
}

fn resolve_project_id(opts: &BootstrapInfisicalCommand, saved: &InfisicalState) -> Result<String> {
    if let Some(id) = &opts.project_id {
        return Ok(id.clone());
    }

    if !std::io::stdin().is_terminal() {
        if let Some(saved_id) = &saved.project_id {
            return Ok(saved_id.clone());
        }
        return Err(anyhow::anyhow!(
            "project_id is required; pass --project-id or INFISICAL_PROJECT_ID"
        ));
    }

    let mut prompt = Text::new("Infisical project ID:");
    if let Some(s) = saved.project_id.as_deref() {
        prompt = prompt.with_default(s);
    }
    let id = map_inquire(prompt.prompt())?;
    let trimmed = id.trim();
    if trimmed.is_empty() {
        Err(anyhow::anyhow!("project ID cannot be empty"))
    } else {
        Ok(trimmed.to_string())
    }
}

fn resolve_client_id(opts: &BootstrapInfisicalCommand) -> Result<String> {
    if let Some(id) = &opts.client_id {
        return Ok(id.clone());
    }

    if !std::io::stdin().is_terminal() {
        return Err(anyhow::anyhow!(
            "client_id is required; pass --client-id or INFISICAL_CLIENT_ID"
        ));
    }

    let id = map_inquire(Text::new("Infisical Universal Auth Client ID:").prompt())?;
    let trimmed = id.trim();
    if trimmed.is_empty() {
        Err(anyhow::anyhow!("client ID cannot be empty"))
    } else {
        Ok(trimmed.to_string())
    }
}

fn resolve_client_secret(opts: &BootstrapInfisicalCommand) -> Result<String> {
    if let Some(secret) = &opts.client_secret {
        return Ok(secret.clone());
    }

    if !std::io::stdin().is_terminal() {
        return Err(anyhow::anyhow!(
            "client_secret is required; pass --client-secret or INFISICAL_CLIENT_SECRET"
        ));
    }

    let secret = map_inquire(Text::new("Infisical Universal Auth Client Secret:").prompt())?;
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        Err(anyhow::anyhow!("client secret cannot be empty"))
    } else {
        Ok(trimmed.to_string())
    }
}

fn resolve_node_name(opts: &BootstrapInfisicalCommand, saved: &InfisicalState) -> Result<String> {
    if let Some(name) = &opts.node_name {
        return Ok(name.clone());
    }
    if let Some(saved_node) = &saved.node_name {
        return Ok(saved_node.clone());
    }

    let output = std::process::Command::new("hostname")
        .output()
        .map_err(|_| anyhow::anyhow!("failed to get hostname; pass --node-name explicitly"))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "hostname command failed; pass --node-name explicitly"
        ));
    }

    let hostname = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if hostname.is_empty() {
        Err(anyhow::anyhow!(
            "hostname is empty; pass --node-name explicitly"
        ))
    } else {
        Ok(hostname)
    }
}

const ADDRESS_EU: &str = "https://eu.infisical.com";
const ADDRESS_US: &str = "https://app.infisical.com";

fn resolve_address(opts: &BootstrapInfisicalCommand, saved: &InfisicalState) -> Result<String> {
    if let Some(addr) = &opts.address {
        return Ok(addr.clone());
    }

    if !std::io::stdin().is_terminal() {
        return Ok(saved
            .address
            .clone()
            .unwrap_or_else(|| ADDRESS_EU.to_string()));
    }

    let options = vec![
        format!("EU ({ADDRESS_EU})"),
        format!("US ({ADDRESS_US})"),
        "Custom (self-hosted)".to_string(),
    ];

    let starting_cursor = match saved.address.as_deref() {
        Some(ADDRESS_EU) => 0,
        Some(ADDRESS_US) => 1,
        Some(_) => 2,
        None => 0,
    };

    let choice = map_inquire(
        Select::new("Infisical region:", options.clone())
            .with_starting_cursor(starting_cursor)
            .prompt(),
    )?;

    if choice == options[0] {
        Ok(ADDRESS_EU.to_string())
    } else if choice == options[1] {
        Ok(ADDRESS_US.to_string())
    } else {
        let mut prompt = Text::new("Infisical API URL:");
        if let Some(saved_addr) = saved.address.as_deref()
            && saved_addr != ADDRESS_EU
            && saved_addr != ADDRESS_US
        {
            prompt = prompt.with_default(saved_addr);
        }
        let url = map_inquire(prompt.prompt())?;
        let trimmed = url.trim().to_string();
        validate::validate_address(&trimmed)?;
        Ok(trimmed)
    }
}

pub fn resolve_inputs(opts: BootstrapInfisicalCommand) -> Result<ResolvedInfisicalInputs> {
    let saved = crate::config::load()
        .ok()
        .and_then(|(cfg, _)| cfg.bootstrap.and_then(|b| b.infisical))
        .unwrap_or_default();

    let address = resolve_address(&opts, &saved)?;
    let environment = opts
        .environment
        .clone()
        .or_else(|| saved.environment.clone())
        .unwrap_or_else(|| "prod".to_string());
    let project_slug = resolve_project_slug(&opts, &saved)?;
    let project_id = resolve_project_id(&opts, &saved)?;
    let node_name = resolve_node_name(&opts, &saved)?;
    let client_id = resolve_client_id(&opts)?;
    let client_secret = resolve_client_secret(&opts)?;

    // Folder discovery moved to planning artifacts stage.
    let folders = opts.folders.clone();

    let secrets_dir = opts
        .secrets_dir
        .unwrap_or_else(|| "/var/secrets".to_string());
    let config_dir = opts
        .config_dir
        .unwrap_or_else(|| "/etc/heimdall/infisical".to_string());

    let config = BootstrapInfisicalConfig {
        address,
        project_slug,
        project_id,
        environment,
        node_name,
        folders,
        client_id,
        client_secret,
        secrets_dir,
        config_dir,
        skip_install: false,
        dry_run: opts.dry_run,
        output: opts.output,
    };

    Ok(ResolvedInfisicalInputs { config })
}
