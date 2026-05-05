use crate::features::bootstrap::infisical::input::BootstrapInfisicalConfig;
use anyhow::Result;
use inquire::Text;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub enum InfisicalPlannedOperation {
    Subprocess {
        id: &'static str,
        description: &'static str,
        command: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
        failure_is_warning: bool,
    },
    WriteFile {
        id: &'static str,
        description: &'static str,
        path: PathBuf,
        content: String,
        mode: u32,
    },
}

#[derive(Debug, Clone)]
pub struct InfisicalPlanArtifacts {
    pub folders: Vec<String>,
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

fn parse_folder_names(stdout_json: &str) -> Result<Vec<String>> {
    let folders = serde_json::from_str::<Vec<serde_json::Value>>(stdout_json)
        .map_err(|_| anyhow::anyhow!("failed to parse folder list JSON"))?;
    Ok(folders
        .iter()
        .filter_map(|f| {
            f.get("folderName")
                .and_then(|n| n.as_str())
                .map(String::from)
        })
        .collect())
}

fn universal_auth_token(client_id: &str, client_secret: &str, address: &str) -> Result<String> {
    let output = Command::new("infisical")
        .args([
            "login",
            "--method=universal-auth",
            "--plain",
            "--silent",
            "--domain",
            address,
            "--client-id",
            client_id,
            "--client-secret",
            client_secret,
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("failed to invoke `infisical login`: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "universal auth login failed: {}",
            stderr.trim()
        ));
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        Err(anyhow::anyhow!(
            "universal auth login returned an empty token"
        ))
    } else {
        Ok(token)
    }
}

pub fn resolve_plan_artifacts(config: &BootstrapInfisicalConfig) -> Result<InfisicalPlanArtifacts> {
    if !config.folders.is_empty() {
        return Ok(InfisicalPlanArtifacts {
            folders: config.folders.clone(),
        });
    }

    let discovery_result =
        universal_auth_token(&config.client_id, &config.client_secret, &config.address).and_then(
            |token| {
                Command::new("infisical")
                    .args([
                        "secrets",
                        "folders",
                        "get",
                        "--domain",
                        &config.address,
                        "--projectId",
                        &config.project_id,
                        "--path",
                        &format!("/{}", config.node_name),
                        "--token",
                        &token,
                        "--output",
                        "json",
                    ])
                    .output()
                    .map_err(|e| anyhow::anyhow!("failed to invoke `infisical`: {e}"))
                    .and_then(|output| {
                        if output.status.success() {
                            parse_folder_names(&String::from_utf8_lossy(&output.stdout))
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            Err(anyhow::anyhow!(
                                "folder discovery failed: {}",
                                stderr.trim()
                            ))
                        }
                    })
            },
        );

    let folders = match discovery_result {
        Ok(folders) => folders,
        Err(err) => {
            eprintln!("warning: folder discovery failed: {err}");
            if !std::io::stdin().is_terminal() {
                Vec::new()
            } else {
                let input = map_inquire(
                    Text::new(
                        "Enter subfolder names (comma-separated), or leave blank for root only:",
                    )
                    .prompt(),
                )?;
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    Vec::new()
                } else {
                    trimmed
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                }
            }
        }
    };

    Ok(InfisicalPlanArtifacts { folders })
}

pub fn build_plan(
    config: &BootstrapInfisicalConfig,
    artifacts: &InfisicalPlanArtifacts,
) -> Result<Vec<InfisicalPlannedOperation>> {
    let mut ops = vec![];

    if !config.skip_install {
        ops.push(InfisicalPlannedOperation::Subprocess {
            id: "download_setup_script",
            description: "Download Infisical RPM setup script",
            command: "curl".to_string(),
            args: vec![
                "-1sLf".to_string(),
                "https://artifacts-cli.infisical.com/setup.rpm.sh".to_string(),
                "-o".to_string(),
                "/tmp/setup.rpm.sh".to_string(),
            ],
            env: vec![],
            failure_is_warning: false,
        });

        ops.push(InfisicalPlannedOperation::Subprocess {
            id: "run_setup_script",
            description: "Configure Infisical RPM repository",
            command: "bash".to_string(),
            args: vec!["/tmp/setup.rpm.sh".to_string()],
            env: vec![],
            failure_is_warning: false,
        });

        ops.push(InfisicalPlannedOperation::Subprocess {
            id: "install_infisical",
            description: "Install Infisical CLI via yum",
            command: "yum".to_string(),
            args: vec![
                "install".to_string(),
                "-y".to_string(),
                "infisical".to_string(),
            ],
            env: vec![],
            failure_is_warning: false,
        });
    }

    let creds_dir = format!("{}/.infisical", config.secrets_dir);

    ops.push(InfisicalPlannedOperation::Subprocess {
        id: "create_creds_dir",
        description: "Create credentials directory",
        command: "mkdir".to_string(),
        args: vec!["-p".to_string(), creds_dir.clone()],
        env: vec![],
        failure_is_warning: false,
    });

    ops.push(InfisicalPlannedOperation::WriteFile {
        id: "write_client_id",
        description: "Write Infisical client ID",
        path: PathBuf::from(format!("{}/client-id", creds_dir)),
        content: config.client_id.clone(),
        mode: 0o600,
    });

    ops.push(InfisicalPlannedOperation::WriteFile {
        id: "write_client_secret",
        description: "Write Infisical client secret",
        path: PathBuf::from(format!("{}/client-secret", creds_dir)),
        content: config.client_secret.clone(),
        mode: 0o600,
    });

    ops.push(InfisicalPlannedOperation::Subprocess {
        id: "secure_creds",
        description: "Set ownership of credentials to root:root",
        command: "chown".to_string(),
        args: vec!["-R".to_string(), "root:root".to_string(), creds_dir],
        env: vec![],
        failure_is_warning: false,
    });

    let config_path = PathBuf::from(&config.config_dir).join("agent.yaml");
    let mut rendered = config.clone();
    rendered.folders = artifacts.folders.clone();
    let agent_yaml_content = crate::features::bootstrap::infisical::generate::agent_yaml(&rendered);

    ops.push(InfisicalPlannedOperation::WriteFile {
        id: "write_agent_config",
        description: "Write Infisical Agent configuration",
        path: config_path.clone(),
        content: agent_yaml_content,
        mode: 0o640,
    });

    let systemd_unit_content = crate::features::bootstrap::infisical::generate::systemd_unit(
        &config_path.display().to_string(),
    );

    ops.push(InfisicalPlannedOperation::WriteFile {
        id: "write_systemd_unit",
        description: "Write systemd service unit",
        path: PathBuf::from("/etc/systemd/system/infisical-agent.service"),
        content: systemd_unit_content,
        mode: 0o644,
    });

    ops.push(InfisicalPlannedOperation::Subprocess {
        id: "verify_universal_auth",
        description: "Verify Universal Auth credentials and project access",
        command: "infisical".to_string(),
        args: vec![
            "secrets".to_string(),
            "--domain".to_string(),
            config.address.clone(),
            "--projectId".to_string(),
            config.project_id.clone(),
            "--env".to_string(),
            config.environment.clone(),
            "--path".to_string(),
            format!("/{}", config.node_name),
            "--silent".to_string(),
        ],
        env: vec![
            (
                "INFISICAL_UNIVERSAL_AUTH_CLIENT_ID".to_string(),
                config.client_id.clone(),
            ),
            (
                "INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET".to_string(),
                config.client_secret.clone(),
            ),
        ],
        failure_is_warning: false,
    });

    ops.push(InfisicalPlannedOperation::Subprocess {
        id: "systemd_daemon_reload",
        description: "Reload systemd daemon",
        command: "systemctl".to_string(),
        args: vec!["daemon-reload".to_string()],
        env: vec![],
        failure_is_warning: false,
    });

    ops.push(InfisicalPlannedOperation::Subprocess {
        id: "systemd_enable",
        description: "Enable infisical-agent service",
        command: "systemctl".to_string(),
        args: vec!["enable".to_string(), "infisical-agent.service".to_string()],
        env: vec![],
        failure_is_warning: false,
    });

    ops.push(InfisicalPlannedOperation::Subprocess {
        id: "systemd_restart",
        description: "Restart infisical-agent service",
        command: "systemctl".to_string(),
        args: vec!["restart".to_string(), "infisical-agent.service".to_string()],
        env: vec![],
        failure_is_warning: false,
    });

    Ok(ops)
}
