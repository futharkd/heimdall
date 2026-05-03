use crate::features::bootstrap::komodo::generate;
use crate::features::bootstrap::komodo::input::BootstrapKomodoConfig;
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum KomodoPlannedOperation {
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
    },
}

pub fn build_plan(config: &BootstrapKomodoConfig) -> Result<Vec<KomodoPlannedOperation>> {
    let mut ops = vec![];
    let dir_path = PathBuf::from(&config.dir);

    // Always check docker and docker compose
    ops.push(KomodoPlannedOperation::Subprocess {
        id: "check_docker",
        description: "Check Docker is installed",
        command: "docker".to_string(),
        args: vec!["--version".to_string()],
        env: vec![],
        failure_is_warning: false,
    });

    ops.push(KomodoPlannedOperation::Subprocess {
        id: "check_docker_compose",
        description: "Check Docker Compose is available",
        command: "docker".to_string(),
        args: vec!["compose".to_string(), "version".to_string()],
        env: vec![],
        failure_is_warning: false,
    });

    if config.mode == crate::cli::KomodoMode::Core {
        // Core mode: write compose.yaml and compose.env, then docker compose up
        let compose_yaml_content = generate::compose_yaml_core(config);
        let compose_env_content = generate::compose_env_core(config);

        ops.push(KomodoPlannedOperation::WriteFile {
            id: "write_compose_yaml",
            description: "Write Docker Compose configuration",
            path: dir_path.join("compose.yaml"),
            content: compose_yaml_content,
        });

        ops.push(KomodoPlannedOperation::WriteFile {
            id: "write_compose_env",
            description: "Write Docker Compose environment variables",
            path: dir_path.join("compose.env"),
            content: compose_env_content,
        });
    } else {
        // Periphery mode
        if let Some(key_content) = &config.core_public_key_content {
            ops.push(KomodoPlannedOperation::WriteFile {
                id: "write_core_public_key",
                description: "Write Core's public key",
                path: dir_path.join("keys").join("core.pub"),
                content: key_content.clone(),
            });
        }

        let compose_yaml_content = generate::compose_yaml_periphery(config);
        let compose_env_content = generate::compose_env_periphery(config);

        ops.push(KomodoPlannedOperation::WriteFile {
            id: "write_compose_yaml",
            description: "Write Docker Compose configuration",
            path: dir_path.join("compose.yaml"),
            content: compose_yaml_content,
        });

        ops.push(KomodoPlannedOperation::WriteFile {
            id: "write_compose_env",
            description: "Write Docker Compose environment variables",
            path: dir_path.join("compose.env"),
            content: compose_env_content,
        });
    }

    // Run docker compose up -d (unless --no-up)
    if !config.no_up {
        ops.push(KomodoPlannedOperation::Subprocess {
            id: "docker_compose_up",
            description: "Start containers with docker compose",
            command: "docker".to_string(),
            args: vec![
                "compose".to_string(),
                "-f".to_string(),
                dir_path.join("compose.yaml").to_string_lossy().to_string(),
                "--env-file".to_string(),
                dir_path.join("compose.env").to_string_lossy().to_string(),
                "up".to_string(),
                "-d".to_string(),
            ],
            env: vec![],
            failure_is_warning: false,
        });
    }

    Ok(ops)
}
