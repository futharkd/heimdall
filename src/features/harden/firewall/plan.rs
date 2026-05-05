use super::input::HardenFirewallConfig;
use super::validate::validate_custom_rule;
use crate::core::operation::{OperationKind, PlannedOperation};
use anyhow::Result;

const FIREWALLD_PACKAGE: &str = "firewalld";

pub fn build_plan(config: &HardenFirewallConfig) -> Result<Vec<PlannedOperation>> {
    let mut operations = Vec::new();

    // Validate all custom rules
    for rule in &config.custom_rules {
        validate_custom_rule(rule)?;
    }

    operations.push(PlannedOperation {
        id: "ensure_firewalld_package",
        description: format!("Install {} package if missing", FIREWALLD_PACKAGE),
        kind: OperationKind::EnsurePackage {
            package: FIREWALLD_PACKAGE.to_string(),
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    // Check firewall-cmd is installed
    operations.push(PlannedOperation {
        id: "probe_firewalld_installed",
        description: "Check firewalld-cmd is installed".to_string(),
        kind: OperationKind::Shell {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "command -v firewall-cmd".to_string()],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    // Check firewalld is active
    operations.push(PlannedOperation {
        id: "probe_firewalld_active",
        description: "Check firewalld service is running".to_string(),
        kind: OperationKind::Shell {
            command: "systemctl".to_string(),
            args: vec!["is-active".to_string(), "firewalld".to_string()],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: true,
        verify: None,
    });

    // Start firewalld if not running
    operations.push(PlannedOperation {
        id: "start_firewalld",
        description: "Start firewalld service".to_string(),
        kind: OperationKind::Shell {
            command: "systemctl".to_string(),
            args: vec!["start".to_string(), "firewalld".to_string()],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: true,
        verify: None,
    });

    // Set default zone to drop (--set-default-zone is stand-alone; do not combine with --permanent).
    operations.push(PlannedOperation {
        id: "set_default_zone_drop",
        description: "Set default firewall zone to drop (deny all inbound)".to_string(),
        kind: OperationKind::Shell {
            command: "firewall-cmd".to_string(),
            args: vec!["--set-default-zone=drop".to_string()],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    // Add presets — rich rules do not support `ct`/conntrack; use `--direct` iptables rules instead.
    if config.allow_established {
        operations.push(PlannedOperation {
            id: "allow_established_ipv4",
            description: "Allow established/related connections (IPv4, direct rule)".to_string(),
            kind: OperationKind::Shell {
                command: "firewall-cmd".to_string(),
                args: vec![
                    "--permanent".to_string(),
                    "--direct".to_string(),
                    "--add-rule".to_string(),
                    "ipv4".to_string(),
                    "filter".to_string(),
                    "INPUT".to_string(),
                    "0".to_string(),
                    "-m".to_string(),
                    "conntrack".to_string(),
                    "--ctstate".to_string(),
                    "RELATED,ESTABLISHED".to_string(),
                    "-j".to_string(),
                    "ACCEPT".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        });
        operations.push(PlannedOperation {
            id: "allow_established_ipv6",
            description: "Allow established/related connections (IPv6, direct rule)".to_string(),
            kind: OperationKind::Shell {
                command: "firewall-cmd".to_string(),
                args: vec![
                    "--permanent".to_string(),
                    "--direct".to_string(),
                    "--add-rule".to_string(),
                    "ipv6".to_string(),
                    "filter".to_string(),
                    "INPUT".to_string(),
                    "0".to_string(),
                    "-m".to_string(),
                    "conntrack".to_string(),
                    "--ctstate".to_string(),
                    "RELATED,ESTABLISHED".to_string(),
                    "-j".to_string(),
                    "ACCEPT".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
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

        operations.push(PlannedOperation {
            id: "allow_ssh",
            description: format!("Allow SSH access (port {})", config.ssh_port),
            kind: OperationKind::Shell {
                command: "firewall-cmd".to_string(),
                args,
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        });
    }

    if config.allow_http {
        operations.push(PlannedOperation {
            id: "allow_http",
            description: "Allow HTTP (port 80)".to_string(),
            kind: OperationKind::Shell {
                command: "firewall-cmd".to_string(),
                args: vec!["--permanent".to_string(), "--add-service=http".to_string()],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        });
    }

    if config.allow_https {
        operations.push(PlannedOperation {
            id: "allow_https",
            description: "Allow HTTPS (port 443)".to_string(),
            kind: OperationKind::Shell {
                command: "firewall-cmd".to_string(),
                args: vec!["--permanent".to_string(), "--add-service=https".to_string()],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        });
    }

    // Add custom rules
    for rule in config.custom_rules.iter() {
        match rule.protocol.as_str() {
            "both" => {
                // For 'both', add two operations: tcp and udp
                operations.push(PlannedOperation {
                    id: "custom_rule_tcp",
                    description: format!("Allow custom port {}/tcp", rule.port),
                    kind: OperationKind::Shell {
                        command: "firewall-cmd".to_string(),
                        args: vec![
                            "--permanent".to_string(),
                            format!("--add-port={}/tcp", rule.port),
                        ],
                        env: vec![],
                        stdin_input: None,
                    },
                    requires_confirmation: false,
                    failure_is_warning: false,
                    verify: None,
                });

                operations.push(PlannedOperation {
                    id: "custom_rule_udp",
                    description: format!("Allow custom port {}/udp", rule.port),
                    kind: OperationKind::Shell {
                        command: "firewall-cmd".to_string(),
                        args: vec![
                            "--permanent".to_string(),
                            format!("--add-port={}/udp", rule.port),
                        ],
                        env: vec![],
                        stdin_input: None,
                    },
                    requires_confirmation: false,
                    failure_is_warning: false,
                    verify: None,
                });
            }
            _ => {
                operations.push(PlannedOperation {
                    id: "custom_rule",
                    description: format!("Allow custom port {}/{}", rule.port, rule.protocol),
                    kind: OperationKind::Shell {
                        command: "firewall-cmd".to_string(),
                        args: vec![
                            "--permanent".to_string(),
                            format!("--add-port={}/{}", rule.port, rule.protocol),
                        ],
                        env: vec![],
                        stdin_input: None,
                    },
                    requires_confirmation: false,
                    failure_is_warning: false,
                    verify: None,
                });
            }
        }
    }

    // Reload firewall
    operations.push(PlannedOperation {
        id: "reload_firewall",
        description: "Reload firewall configuration".to_string(),
        kind: OperationKind::Shell {
            command: "firewall-cmd".to_string(),
            args: vec!["--reload".to_string()],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    Ok(operations)
}
