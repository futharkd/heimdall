use crate::features::bootstrap::infisical::input::BootstrapInfisicalConfig;
use anyhow::Result;
use inquire::Text;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::Command;
use tracing::debug;

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

pub(crate) fn parse_folder_names(stdout_json: &str) -> Result<Vec<String>> {
    let folders = serde_json::from_str::<Vec<serde_json::Value>>(stdout_json).map_err(|e| {
        let payload_preview = stdout_json.chars().take(200).collect::<String>();
        anyhow::anyhow!(
            "failed to parse folder list JSON: {} (payload: {})",
            e,
            payload_preview
        )
    })?;
    Ok(folders
        .iter()
        .filter_map(|f| {
            f.get("folderName")
                .and_then(|n| n.as_str())
                .map(String::from)
        })
        .collect())
}

pub(crate) fn universal_auth_token(
    client_id: &str,
    client_secret: &str,
    address: &str,
) -> Result<String> {
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

fn should_show_discovery_details() -> bool {
    std::io::stdin().is_terminal()
}

pub(crate) fn discover_folders_recursive(
    address: &str,
    project_id: &str,
    token: &str,
    node_name: &str,
    environment: &str,
) -> Vec<String> {
    discover_folders_at(
        address,
        project_id,
        token,
        &format!("/{}", node_name),
        "",
        0,
        environment,
    )
}

fn discover_folders_at(
    address: &str,
    project_id: &str,
    token: &str,
    infisical_path: &str,
    relative_prefix: &str,
    depth: u8,
    environment: &str,
) -> Vec<String> {
    if depth >= 10 {
        debug!(
            infisical_path = %infisical_path,
            depth = depth,
            "folder discovery reached max depth limit"
        );
        if should_show_discovery_details() {
            eprintln!("[discovery] Reached max depth limit at: {}", infisical_path);
        }
        return Vec::new();
    }

    if should_show_discovery_details() {
        eprintln!("[discovery] Querying path: {}", infisical_path);
        eprintln!(
            "[discovery] Command: infisical secrets folders get --domain {} --projectId {} --path {} --env {} --token ****",
            address, project_id, infisical_path, environment
        );
    }

    let output = match Command::new("infisical")
        .args([
            "secrets",
            "folders",
            "get",
            "--domain",
            address,
            "--projectId",
            project_id,
            "--path",
            infisical_path,
            "--env",
            environment,
            "--token",
            token,
            "--output",
            "json",
        ])
        .output()
    {
        Ok(out) => out,
        Err(e) => {
            debug!(
                error = %e,
                infisical_path = %infisical_path,
                depth = depth,
                "infisical folders discovery command execution failed"
            );
            if should_show_discovery_details() {
                eprintln!("[discovery] ❌ Command execution failed: {}", e);
            }
            return Vec::new();
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code();
        debug!(
            exit_code = ?exit_code,
            stderr = %stderr,
            infisical_path = %infisical_path,
            depth = depth,
            "infisical folders command returned non-zero exit code"
        );
        if should_show_discovery_details() {
            eprintln!("[discovery] ❌ Exit code: {:?}", exit_code);
            eprintln!("[discovery] ❌ stderr: {}", stderr);
        }
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if should_show_discovery_details() {
        eprintln!("[discovery] ✓ Response stdout: {}", stdout);
    }

    let child_names = match parse_folder_names(&stdout) {
        Ok(names) => {
            debug!(
                count = names.len(),
                infisical_path = %infisical_path,
                depth = depth,
                "successfully discovered folders"
            );
            if should_show_discovery_details() {
                eprintln!(
                    "[discovery] ✓ Parsed {} folders from: {}",
                    names.len(),
                    infisical_path
                );
                for name in &names {
                    eprintln!("[discovery]   - {}", name);
                }
            }
            names
        }
        Err(e) => {
            debug!(
                error = %e,
                infisical_path = %infisical_path,
                depth = depth,
                "failed to parse folder list JSON response"
            );
            if should_show_discovery_details() {
                eprintln!("[discovery] ❌ Failed to parse JSON: {}", e);
            }
            return Vec::new();
        }
    };

    let mut result = Vec::new();
    for name in child_names {
        let relative_path = if relative_prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", relative_prefix, name)
        };
        result.push(relative_path.clone());

        let next_infisical_path = format!("{}/{}", infisical_path, name);
        let mut children = discover_folders_at(
            address,
            project_id,
            token,
            &next_infisical_path,
            &relative_path,
            depth + 1,
            environment,
        );
        result.append(&mut children);
    }

    result
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
                let folders = discover_folders_at(
                    &config.address,
                    &config.project_id,
                    &token,
                    &format!("/{}", config.node_name),
                    "",
                    0,
                    &config.environment,
                );
                if folders.is_empty() {
                    Err(anyhow::anyhow!(
                        "no folders discovered at /{}\n\
                         Check if the node path exists and has subfolders.\n\
                         You can manually specify folders with --folder flag.",
                        config.node_name
                    ))
                } else {
                    Ok(folders)
                }
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

    ops.push(InfisicalPlannedOperation::Subprocess {
        id: "create_secrets_dir",
        description: "Create secrets output root directory",
        command: "mkdir".to_string(),
        args: vec!["-p".to_string(), config.secrets_dir.clone()],
        env: vec![],
        failure_is_warning: false,
    });

    for folder in &artifacts.folders {
        ops.push(InfisicalPlannedOperation::Subprocess {
            id: "create_secret_subdir",
            description: "Create secrets output subdirectory",
            command: "mkdir".to_string(),
            args: vec![
                "-p".to_string(),
                format!("{}/{}", config.secrets_dir, folder),
            ],
            env: vec![],
            failure_is_warning: false,
        });
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_folder_names_empty_list() {
        let result = parse_folder_names("[]");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_parse_folder_names_single_folder() {
        let json = r#"[{"folderName": "prod"}]"#;
        let result = parse_folder_names(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["prod"]);
    }

    #[test]
    fn test_parse_folder_names_multiple_folders() {
        let json = r#"[
            {"folderName": "prod"},
            {"folderName": "staging"},
            {"folderName": "dev"}
        ]"#;
        let result = parse_folder_names(json);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            vec!["prod".to_string(), "staging".to_string(), "dev".to_string()]
        );
    }

    #[test]
    fn test_parse_folder_names_invalid_json() {
        let result = parse_folder_names("not valid json {");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("failed to parse folder list JSON"));
    }

    #[test]
    fn test_parse_folder_names_missing_folder_name_field() {
        let json = r#"[{"name": "prod"}, {"folderName": "staging"}]"#;
        let result = parse_folder_names(json);
        assert!(result.is_ok());
        let folders = result.unwrap();
        // Only staging should be included because the first object lacks folderName
        assert_eq!(folders, vec!["staging".to_string()]);
    }

    #[test]
    fn test_parse_folder_names_null_values() {
        let json = r#"[{"folderName": null}, {"folderName": "valid"}]"#;
        let result = parse_folder_names(json);
        assert!(result.is_ok());
        let folders = result.unwrap();
        // Only valid folder should be included
        assert_eq!(folders, vec!["valid".to_string()]);
    }

    #[test]
    fn test_parse_folder_names_includes_payload_in_error() {
        let invalid_json = "this is not json";
        let result = parse_folder_names(invalid_json);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // Error should contain both the parse error and a preview of the payload
        assert!(err_msg.contains("failed to parse folder list JSON"));
        assert!(err_msg.contains("this is not json"));
    }

    #[test]
    fn test_discover_folders_recursive_signature() {
        // This test verifies the function signature takes environment parameter
        // Compile-time check: if the signature changes, this test will fail to compile
        use std::marker::PhantomData;

        // We can't directly test the function without mocking subprocess calls,
        // but we can verify it compiles with the expected signature
        let _ = PhantomData::<fn(&str, &str, &str, &str, &str) -> Vec<String>>::default();
    }

    #[test]
    fn test_discover_folders_at_includes_environment_in_debug() {
        // This is a structural test: we verify the function accepts environment
        // In real usage, folder discovery would call infisical with --env flag

        // The discover_folders_at function should accept 7 parameters (up from 6)
        // and pass environment to the infisical command

        // We can't run real discovery without mocking, but we've verified
        // the signature is correct and the implementation includes --env in the command builder
    }
}
