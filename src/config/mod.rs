use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HeimdallConfig {
    pub harden: Option<HardenConfig>,
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
        let content = fs::read_to_string(&etc_path)?;
        let config: HeimdallConfig = serde_yaml::from_str(&content)?;
        return Ok((config, etc_path));
    }

    // Try ~/.heimdall/config.yaml
    if let Some(ref path) = home_path {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let config: HeimdallConfig = serde_yaml::from_str(&content)?;
            return Ok((config, path.clone()));
        }
    }

    // Neither exists; return default + preferred write location
    let write_path = etc_path;
    Ok((HeimdallConfig::default(), write_path))
}

/// Save config to the specified path.
/// Creates parent directories if needed.
pub fn save(config: &HeimdallConfig, path: &Path) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let yaml = serde_yaml::to_string(config)?;
    fs::write(path, yaml)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialize_deserialize() {
        let mut config = HeimdallConfig::default();
        config.harden = Some(HardenConfig {
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
        });

        let yaml = serde_yaml::to_string(&config).expect("serialize");
        let parsed: HeimdallConfig = serde_yaml::from_str(&yaml).expect("deserialize");

        assert!(parsed.harden.is_some());
        let harden = parsed.harden.unwrap();
        assert!(harden.ssh.is_some());
        assert_eq!(harden.ssh.unwrap().port, Some(2222));
        assert!(harden.firewall.is_some());
    }
}
