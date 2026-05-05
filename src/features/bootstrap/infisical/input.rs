use crate::cli::{BootstrapInfisicalCommand, OutputFormat};
use anyhow::Result;
use inquire::Text;
use std::io::IsTerminal;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct BootstrapInfisicalConfig {
    pub address: String,
    pub project_slug: String,
    pub environment: String,
    pub node_name: String,
    pub folders: Vec<String>,
    pub client_id: String,
    pub client_secret: String,
    pub secrets_dir: String,
    pub config_dir: String,
    pub skip_login: bool,
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

fn resolve_project_slug(opts: &BootstrapInfisicalCommand) -> Result<String> {
    if let Some(slug) = &opts.project_slug {
        return Ok(slug.clone());
    }

    if !std::io::stdin().is_terminal() {
        return Err(anyhow::anyhow!(
            "project_slug is required; pass --project-slug or INFISICAL_PROJECT_SLUG"
        ));
    }

    let slug = map_inquire(Text::new("Infisical project slug:").prompt())?;
    let trimmed = slug.trim();
    if trimmed.is_empty() {
        Err(anyhow::anyhow!("project slug cannot be empty"))
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

fn resolve_node_name(opts: &BootstrapInfisicalCommand) -> Result<String> {
    if let Some(name) = &opts.node_name {
        return Ok(name.clone());
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

fn discover_folders(project_slug: &str, environment: &str, node_name: &str) -> Result<Vec<String>> {
    let output = Command::new("infisical")
        .args([
            "secrets",
            "folders",
            "list",
            "--project-slug",
            project_slug,
            "--env",
            environment,
            "--path",
            &format!("/{}", node_name),
            "--json",
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            match serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                Ok(folders) => {
                    let names: Vec<String> = folders
                        .iter()
                        .filter_map(|f| f.get("name").and_then(|n| n.as_str()).map(String::from))
                        .collect();
                    Ok(names)
                }
                Err(_) => Err(anyhow::anyhow!("failed to parse folder list JSON")),
            }
        }
        _ => Err(anyhow::anyhow!("folder discovery failed")),
    }
}

fn resolve_folders(
    opts: &BootstrapInfisicalCommand,
    project_slug: &str,
    environment: &str,
    node_name: &str,
) -> Result<Vec<String>> {
    if !opts.folders.is_empty() {
        return Ok(opts.folders.clone());
    }

    match discover_folders(project_slug, environment, node_name) {
        Ok(folders) => Ok(folders),
        Err(_) => {
            if !std::io::stdin().is_terminal() {
                return Ok(vec![]);
            }

            let input = map_inquire(
                Text::new("Enter subfolder names (comma-separated), or leave blank for root only:")
                    .prompt(),
            )?;
            let trimmed = input.trim();
            if trimmed.is_empty() {
                Ok(vec![])
            } else {
                Ok(trimmed
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect())
            }
        }
    }
}

pub fn resolve_inputs(opts: BootstrapInfisicalCommand) -> Result<ResolvedInfisicalInputs> {
    let address = opts
        .address
        .clone()
        .unwrap_or_else(|| "https://eu.infisical.com".to_string());
    let environment = opts
        .environment
        .clone()
        .unwrap_or_else(|| "prod".to_string());
    let project_slug = resolve_project_slug(&opts)?;
    let node_name = resolve_node_name(&opts)?;
    let client_id = resolve_client_id(&opts)?;
    let client_secret = resolve_client_secret(&opts)?;

    let folders = resolve_folders(&opts, &project_slug, &environment, &node_name)?;

    let secrets_dir = opts
        .secrets_dir
        .unwrap_or_else(|| "/var/secrets".to_string());
    let config_dir = opts
        .config_dir
        .unwrap_or_else(|| "/etc/heimdall/infisical".to_string());

    let config = BootstrapInfisicalConfig {
        address,
        project_slug,
        environment,
        node_name,
        folders,
        client_id,
        client_secret,
        secrets_dir,
        config_dir,
        skip_login: opts.skip_login,
        skip_install: false,
        dry_run: opts.dry_run,
        output: opts.output,
    };

    Ok(ResolvedInfisicalInputs { config })
}
