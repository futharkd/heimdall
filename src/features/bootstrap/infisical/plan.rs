use crate::features::bootstrap::infisical::input::BootstrapInfisicalConfig;
use anyhow::Result;
use std::path::PathBuf;

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

pub fn build_plan(config: &BootstrapInfisicalConfig) -> Result<Vec<InfisicalPlannedOperation>> {
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
            command: "sudo".to_string(),
            args: vec!["bash".to_string(), "/tmp/setup.rpm.sh".to_string()],
            env: vec![],
            failure_is_warning: false,
        });

        ops.push(InfisicalPlannedOperation::Subprocess {
            id: "install_infisical",
            description: "Install Infisical CLI via yum",
            command: "sudo".to_string(),
            args: vec![
                "yum".to_string(),
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
        command: "sudo".to_string(),
        args: vec!["mkdir".to_string(), "-p".to_string(), creds_dir.clone()],
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
        command: "sudo".to_string(),
        args: vec![
            "chown".to_string(),
            "-R".to_string(),
            "root:root".to_string(),
            creds_dir,
        ],
        env: vec![],
        failure_is_warning: false,
    });

    let config_path = PathBuf::from(&config.config_dir).join("agent.yaml");
    let agent_yaml_content = crate::features::bootstrap::infisical::generate::agent_yaml(config);

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
        command: "sudo".to_string(),
        args: vec!["systemctl".to_string(), "daemon-reload".to_string()],
        env: vec![],
        failure_is_warning: false,
    });

    ops.push(InfisicalPlannedOperation::Subprocess {
        id: "systemd_enable",
        description: "Enable infisical-agent service",
        command: "sudo".to_string(),
        args: vec![
            "systemctl".to_string(),
            "enable".to_string(),
            "infisical-agent.service".to_string(),
        ],
        env: vec![],
        failure_is_warning: false,
    });

    ops.push(InfisicalPlannedOperation::Subprocess {
        id: "systemd_restart",
        description: "Restart infisical-agent service",
        command: "sudo".to_string(),
        args: vec![
            "systemctl".to_string(),
            "restart".to_string(),
            "infisical-agent.service".to_string(),
        ],
        env: vec![],
        failure_is_warning: false,
    });

    Ok(ops)
}
