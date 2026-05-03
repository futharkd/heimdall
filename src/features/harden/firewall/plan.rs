use super::input::HardenFirewallConfig;
use super::validate::validate_custom_rule;
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FirewallPlannedOperation {
    pub id: String,
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub failure_is_warning: bool,
}

pub fn build_plan(config: &HardenFirewallConfig) -> Result<Vec<FirewallPlannedOperation>> {
    let mut operations = Vec::new();

    // Validate all custom rules
    for rule in &config.custom_rules {
        validate_custom_rule(rule)?;
    }

    // Check firewall-cmd is installed
    operations.push(FirewallPlannedOperation {
        id: "probe_firewalld_installed".to_string(),
        description: "Check firewalld-cmd is installed".to_string(),
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "command -v firewall-cmd".to_string()],
        env: vec![],
        failure_is_warning: false,
    });

    // Check firewalld is active
    operations.push(FirewallPlannedOperation {
        id: "probe_firewalld_active".to_string(),
        description: "Check firewalld service is running".to_string(),
        command: "systemctl".to_string(),
        args: vec!["is-active".to_string(), "firewalld".to_string()],
        env: vec![],
        failure_is_warning: true,
    });

    // Start firewalld if not running
    operations.push(FirewallPlannedOperation {
        id: "start_firewalld".to_string(),
        description: "Start firewalld service".to_string(),
        command: "systemctl".to_string(),
        args: vec!["start".to_string(), "firewalld".to_string()],
        env: vec![],
        failure_is_warning: true,
    });

    // Set default zone to drop
    operations.push(FirewallPlannedOperation {
        id: "set_default_zone_drop".to_string(),
        description: "Set default firewall zone to drop (deny all inbound)".to_string(),
        command: "firewall-cmd".to_string(),
        args: vec![
            "--permanent".to_string(),
            "--set-default-zone=drop".to_string(),
        ],
        env: vec![],
        failure_is_warning: false,
    });

    // Add presets
    if config.allow_established {
        operations.push(FirewallPlannedOperation {
            id: "allow_established".to_string(),
            description: "Allow established/related connections".to_string(),
            command: "firewall-cmd".to_string(),
            args: vec![
                "--permanent".to_string(),
                "--add-rich-rule=rule family=\"ipv4\" ct state established,related accept"
                    .to_string(),
            ],
            env: vec![],
            failure_is_warning: false,
        });
    }

    if config.allow_ssh {
        let args = if config.ssh_port == 22 {
            vec!["--permanent".to_string(), "--add-service=ssh".to_string()]
        } else {
            vec![
                "--permanent".to_string(),
                format!("--add-port={}/tcp", config.ssh_port),
            ]
        };

        operations.push(FirewallPlannedOperation {
            id: "allow_ssh".to_string(),
            description: format!("Allow SSH access (port {})", config.ssh_port),
            command: "firewall-cmd".to_string(),
            args,
            env: vec![],
            failure_is_warning: false,
        });
    }

    if config.allow_http {
        operations.push(FirewallPlannedOperation {
            id: "allow_http".to_string(),
            description: "Allow HTTP (port 80)".to_string(),
            command: "firewall-cmd".to_string(),
            args: vec![
                "--permanent".to_string(),
                "--add-service=http".to_string(),
            ],
            env: vec![],
            failure_is_warning: false,
        });
    }

    if config.allow_https {
        operations.push(FirewallPlannedOperation {
            id: "allow_https".to_string(),
            description: "Allow HTTPS (port 443)".to_string(),
            command: "firewall-cmd".to_string(),
            args: vec![
                "--permanent".to_string(),
                "--add-service=https".to_string(),
            ],
            env: vec![],
            failure_is_warning: false,
        });
    }

    // Add custom rules
    for rule in config.custom_rules.iter() {
        match rule.protocol.as_str() {
            "both" => {
                // For 'both', add two operations: tcp and udp
                operations.push(FirewallPlannedOperation {
                    id: "custom_rule_tcp".to_string(),
                    description: format!("Allow custom port {}/tcp", rule.port),
                    command: "firewall-cmd".to_string(),
                    args: vec![
                        "--permanent".to_string(),
                        format!("--add-port={}/tcp", rule.port),
                    ],
                    env: vec![],
                    failure_is_warning: false,
                });

                operations.push(FirewallPlannedOperation {
                    id: "custom_rule_udp".to_string(),
                    description: format!("Allow custom port {}/udp", rule.port),
                    command: "firewall-cmd".to_string(),
                    args: vec![
                        "--permanent".to_string(),
                        format!("--add-port={}/udp", rule.port),
                    ],
                    env: vec![],
                    failure_is_warning: false,
                });
            }
            _ => {
                operations.push(FirewallPlannedOperation {
                    id: "custom_rule".to_string(),
                    description: format!("Allow custom port {}/{}", rule.port, rule.protocol),
                    command: "firewall-cmd".to_string(),
                    args: vec![
                        "--permanent".to_string(),
                        format!("--add-port={}/{}", rule.port, rule.protocol),
                    ],
                    env: vec![],
                    failure_is_warning: false,
                });
            }
        }
    }

    // Reload firewall
    operations.push(FirewallPlannedOperation {
        id: "reload_firewall".to_string(),
        description: "Reload firewall configuration".to_string(),
        command: "firewall-cmd".to_string(),
        args: vec!["--reload".to_string()],
        env: vec![],
        failure_is_warning: false,
    });

    Ok(operations)
}
