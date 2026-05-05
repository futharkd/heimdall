use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::core::operation::OperationStatus;
use crate::runner::read::read_file_with_escalation;
use crate::runner::write::write_file_with_escalation;
use crate::runner::{IoMode, LocalRunner};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HeimdallConfig {
    pub harden: Option<HardenConfig>,
    pub bootstrap: Option<BootstrapConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BootstrapConfig {
    pub infisical: Option<InfisicalState>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct InfisicalState {
    pub address: Option<String>,
    pub project_id: Option<String>,
    pub project_slug: Option<String>,
    pub environment: Option<String>,
    pub node_name: Option<String>,
    #[serde(default)]
    pub secrets_dir: Option<String>,
    #[serde(default)]
    pub config_dir: Option<String>,
    #[serde(default)]
    pub folders: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HardenConfig {
    pub ssh: Option<SshHardenState>,
    pub firewall: Option<FirewallHardenState>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SshHardenState {
    pub port: Option<u16>,
    pub root_login_disabled: bool,
    pub password_auth_disabled: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FirewallHardenState {
    pub applied: bool,
    pub presets: Vec<String>,
    pub custom_rules: Vec<CustomFirewallRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFirewallRule {
    pub port: u16,
    pub protocol: String,
}

/// Attempt to load config from standard locations.
/// Tries /etc/heimdall/config.yaml first, falls back to ~/.heimdall/config.yaml.
/// If neither exists, returns default config with the location where it would be written.
pub fn load() -> Result<(HeimdallConfig, PathBuf)> {
    let etc_path = PathBuf::from("/etc/heimdall/config.yaml");
    let home_path = {
        let home = std::env::var("HOME").ok();
        home.map(|h| PathBuf::from(h).join(".heimdall/config.yaml"))
    };

    // Try /etc/heimdall/config.yaml first
    if etc_path.exists() {
        let runner = LocalRunner;
        let content = read_file_with_escalation(&runner, &etc_path, IoMode::Buffered)?;
        let config: HeimdallConfig = serde_yaml::from_str(&content)?;
        return Ok((config, etc_path));
    }

    // Try ~/.heimdall/config.yaml
    if let Some(ref path) = home_path
        && path.exists()
    {
        let content = fs::read_to_string(path)?;
        let config: HeimdallConfig = serde_yaml::from_str(&content)?;
        return Ok((config, path.clone()));
    }

    // Neither exists; return default + preferred write location
    let write_path = etc_path;
    Ok((HeimdallConfig::default(), write_path))
}

/// Save config to the specified path.
/// Creates parent directories if needed.
pub fn save(config: &HeimdallConfig, path: &Path) -> Result<()> {
    let yaml = serde_yaml::to_string(config)?;
    let runner = LocalRunner;
    let status = write_file_with_escalation(&runner, path, &yaml, Some(0o600), IoMode::Buffered);
    match status {
        OperationStatus::Succeeded => Ok(()),
        _ => Err(anyhow::anyhow!(
            "failed to write config at {}",
            path.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_infisical_state_roundtrip() {
        let config = HeimdallConfig {
            harden: None,
            bootstrap: Some(BootstrapConfig {
                infisical: Some(InfisicalState {
                    address: Some("https://eu.infisical.com".to_string()),
                    project_id: Some("5bafc061-6f3a-4e06-aa0e-43f9be261aab".to_string()),
                    project_slug: Some("kenaz".to_string()),
                    environment: Some("prod".to_string()),
                    node_name: Some("kenaz".to_string()),
                    secrets_dir: Some("/var/secrets".to_string()),
                    config_dir: Some("/etc/heimdall/infisical".to_string()),
                    folders: vec!["app".to_string(), "app/config".to_string()],
                }),
            }),
        };

        let yaml = serde_yaml::to_string(&config).expect("serialize");
        let parsed: HeimdallConfig = serde_yaml::from_str(&yaml).expect("deserialize");

        let infisical = parsed
            .bootstrap
            .expect("bootstrap")
            .infisical
            .expect("infisical");
        assert_eq!(
            infisical.project_id.as_deref(),
            Some("5bafc061-6f3a-4e06-aa0e-43f9be261aab")
        );
        assert_eq!(infisical.project_slug.as_deref(), Some("kenaz"));
        assert_eq!(
            infisical.address.as_deref(),
            Some("https://eu.infisical.com")
        );
        assert_eq!(infisical.environment.as_deref(), Some("prod"));
        assert_eq!(infisical.node_name.as_deref(), Some("kenaz"));
        assert_eq!(infisical.secrets_dir.as_deref(), Some("/var/secrets"));
        assert_eq!(
            infisical.config_dir.as_deref(),
            Some("/etc/heimdall/infisical")
        );
        assert_eq!(infisical.folders.len(), 2);
    }

    #[test]
    fn test_config_serialize_deserialize() {
        let config = HeimdallConfig {
            harden: Some(HardenConfig {
                ssh: Some(SshHardenState {
                    port: Some(2222),
                    root_login_disabled: true,
                    password_auth_disabled: false,
                }),
                firewall: Some(FirewallHardenState {
                    applied: true,
                    presets: vec!["ssh".to_string(), "established".to_string()],
                    custom_rules: vec![CustomFirewallRule {
                        port: 8080,
                        protocol: "tcp".to_string(),
                    }],
                }),
            }),
            bootstrap: None,
        };

        let yaml = serde_yaml::to_string(&config).expect("serialize");
        let parsed: HeimdallConfig = serde_yaml::from_str(&yaml).expect("deserialize");

        assert!(parsed.harden.is_some());
        let harden = parsed.harden.unwrap();
        assert!(harden.ssh.is_some());
        assert_eq!(harden.ssh.unwrap().port, Some(2222));
        assert!(harden.firewall.is_some());
    }
}
