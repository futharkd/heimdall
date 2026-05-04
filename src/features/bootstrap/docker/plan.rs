use std::path::PathBuf;

use crate::features::bootstrap::docker::generate;
use crate::features::bootstrap::docker::input::DockerConfig;

#[derive(Debug, Clone)]
pub enum DockerPlannedOperation {
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

pub fn build_plan(config: &DockerConfig) -> anyhow::Result<Vec<DockerPlannedOperation>> {
    let mut operations = Vec::new();

    if !config.skip_install {
        let (command, args) = build_install_command(&config.install_script_url);
        operations.push(DockerPlannedOperation::Subprocess {
            id: "install_docker",
            description: "Download and run Docker install script",
            command,
            args,
            env: vec![],
            failure_is_warning: false,
        });
    }

    operations.push(DockerPlannedOperation::Subprocess {
        id: "enable_docker_service",
        description: "Enable and start docker systemd service",
        command: "systemctl".to_string(),
        args: vec![
            "enable".to_string(),
            "--now".to_string(),
            "docker".to_string(),
        ],
        env: vec![],
        failure_is_warning: false,
    });

    if config.log_driver.is_some() || !config.registry_mirrors.is_empty() {
        let daemon_json =
            generate::generate_daemon_json(config.log_driver.as_deref(), &config.registry_mirrors)?;

        operations.push(DockerPlannedOperation::WriteFile {
            id: "write_daemon_json",
            description: "Write /etc/docker/daemon.json",
            path: PathBuf::from("/etc/docker/daemon.json"),
            content: daemon_json,
        });
    }

    if let Some(ref user) = config.add_user {
        operations.push(DockerPlannedOperation::Subprocess {
            id: "add_to_docker_group",
            description: "Add user to docker group",
            command: "usermod".to_string(),
            args: vec!["-aG".to_string(), "docker".to_string(), user.clone()],
            env: vec![],
            failure_is_warning: false,
        });
    }

    operations.push(DockerPlannedOperation::Subprocess {
        id: "verify_docker",
        description: "Verify Docker daemon responds",
        command: "docker".to_string(),
        args: vec!["info".to_string()],
        env: vec![],
        failure_is_warning: true,
    });

    Ok(operations)
}

fn build_install_command(url: &str) -> (String, Vec<String>) {
    (
        "sh".to_string(),
        vec!["-c".to_string(), format!("curl -fsSL {} | sh", url)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_plan_skips_install_when_skip_install_true() {
        let config = DockerConfig {
            install_script_url: "https://get.docker.com".to_string(),
            add_user: None,
            log_driver: None,
            registry_mirrors: vec![],
            dry_run: false,
            force: false,
            skip_install: true,
        };

        let ops = build_plan(&config).unwrap();
        assert!(!ops.iter().any(|op| matches!(
            op,
            DockerPlannedOperation::Subprocess {
                id: "install_docker",
                ..
            }
        )));
    }

    #[test]
    fn build_plan_includes_install_when_skip_install_false() {
        let config = DockerConfig {
            install_script_url: "https://get.docker.com".to_string(),
            add_user: None,
            log_driver: None,
            registry_mirrors: vec![],
            dry_run: false,
            force: false,
            skip_install: false,
        };

        let ops = build_plan(&config).unwrap();
        assert!(ops.iter().any(|op| matches!(
            op,
            DockerPlannedOperation::Subprocess {
                id: "install_docker",
                ..
            }
        )));
    }

    #[test]
    fn build_plan_omits_daemon_json_when_no_config() {
        let config = DockerConfig {
            install_script_url: "https://get.docker.com".to_string(),
            add_user: None,
            log_driver: None,
            registry_mirrors: vec![],
            dry_run: false,
            force: false,
            skip_install: true,
        };

        let ops = build_plan(&config).unwrap();
        assert!(!ops.iter().any(|op| matches!(
            op,
            DockerPlannedOperation::WriteFile {
                id: "write_daemon_json",
                ..
            }
        )));
    }

    #[test]
    fn build_plan_includes_daemon_json_with_log_driver() {
        let config = DockerConfig {
            install_script_url: "https://get.docker.com".to_string(),
            add_user: None,
            log_driver: Some("json-file".to_string()),
            registry_mirrors: vec![],
            dry_run: false,
            force: false,
            skip_install: true,
        };

        let ops = build_plan(&config).unwrap();
        assert!(ops.iter().any(|op| matches!(
            op,
            DockerPlannedOperation::WriteFile {
                id: "write_daemon_json",
                ..
            }
        )));
    }

    #[test]
    fn build_plan_includes_daemon_json_with_registry_mirrors() {
        let config = DockerConfig {
            install_script_url: "https://get.docker.com".to_string(),
            add_user: None,
            log_driver: None,
            registry_mirrors: vec!["https://mirror.example.com".to_string()],
            dry_run: false,
            force: false,
            skip_install: true,
        };

        let ops = build_plan(&config).unwrap();
        assert!(ops.iter().any(|op| matches!(
            op,
            DockerPlannedOperation::WriteFile {
                id: "write_daemon_json",
                ..
            }
        )));
    }

    #[test]
    fn build_plan_omits_add_user_when_none() {
        let config = DockerConfig {
            install_script_url: "https://get.docker.com".to_string(),
            add_user: None,
            log_driver: None,
            registry_mirrors: vec![],
            dry_run: false,
            force: false,
            skip_install: true,
        };

        let ops = build_plan(&config).unwrap();
        assert!(!ops.iter().any(|op| matches!(
            op,
            DockerPlannedOperation::Subprocess {
                id: "add_to_docker_group",
                ..
            }
        )));
    }

    #[test]
    fn build_plan_includes_add_user_when_some() {
        let config = DockerConfig {
            install_script_url: "https://get.docker.com".to_string(),
            add_user: Some("ubuntu".to_string()),
            log_driver: None,
            registry_mirrors: vec![],
            dry_run: false,
            force: false,
            skip_install: true,
        };

        let ops = build_plan(&config).unwrap();
        assert!(ops.iter().any(|op| matches!(
            op,
            DockerPlannedOperation::Subprocess {
                id: "add_to_docker_group",
                ..
            }
        )));
    }

    #[test]
    fn build_plan_always_includes_enable_and_verify() {
        let config = DockerConfig {
            install_script_url: "https://get.docker.com".to_string(),
            add_user: None,
            log_driver: None,
            registry_mirrors: vec![],
            dry_run: false,
            force: false,
            skip_install: true,
        };

        let ops = build_plan(&config).unwrap();
        assert!(ops.iter().any(|op| matches!(
            op,
            DockerPlannedOperation::Subprocess {
                id: "enable_docker_service",
                ..
            }
        )));
        assert!(ops.iter().any(|op| matches!(
            op,
            DockerPlannedOperation::Subprocess {
                id: "verify_docker",
                ..
            }
        )));
    }
}
