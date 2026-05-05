use crate::config;

use super::super::report::{CheckStatus, DoctorCheck};

pub fn contribute() -> Vec<DoctorCheck> {
    match config::load() {
        Err(e) => vec![DoctorCheck {
            id: "heimdall_config",
            description: "Load Heimdall config YAML",
            status: CheckStatus::Fail,
            detail: format!("{e:#}"),
        }],
        Ok((cfg, path)) => {
            let mut detail = String::new();
            if path.exists() {
                detail.push_str(&format!("path: {}\n", path.display()));
            } else {
                detail.push_str(&format!(
                    "no config file yet (default write location: {})\n",
                    path.display()
                ));
            }

            if let Some(ref h) = cfg.harden {
                if let Some(ref ssh) = h.ssh {
                    detail.push_str(&format!(
                        "harden.ssh: port={:?}, root_login_disabled={}, password_auth_disabled={}\n",
                        ssh.port, ssh.root_login_disabled, ssh.password_auth_disabled
                    ));
                }
                if let Some(ref fw) = h.firewall {
                    detail.push_str(&format!(
                        "harden.firewall: applied={}, presets={}, custom_rules={}\n",
                        fw.applied,
                        fw.presets.len(),
                        fw.custom_rules.len()
                    ));
                }
            }

            if let Some(ref b) = cfg.bootstrap
                && let Some(ref inf) = b.infisical
            {
                detail.push_str("bootstrap.infisical: ");
                if let Some(ref a) = inf.address {
                    detail.push_str(&format!("address={a} "));
                }
                if let Some(ref slug) = inf.project_slug {
                    detail.push_str(&format!("project_slug={slug} "));
                }
                if let Some(ref env) = inf.environment {
                    detail.push_str(&format!("environment={env} "));
                }
                if let Some(ref node) = inf.node_name {
                    detail.push_str(&format!("node_name={node} "));
                }
                detail.push('\n');
            }

            let status = if path.exists() {
                CheckStatus::Pass
            } else {
                CheckStatus::Warn
            };

            vec![DoctorCheck {
                id: "heimdall_config",
                description: "Heimdall persisted config state",
                status,
                detail: detail.trim_end().to_string(),
            }]
        }
    }
}
