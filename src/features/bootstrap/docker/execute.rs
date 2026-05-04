use crate::core::operation::{OperationResult, OperationStatus};
use crate::features::bootstrap::docker::input::DockerConfig;
use crate::features::bootstrap::docker::plan::DockerPlannedOperation;
use crate::features::bootstrap::docker::report::BootstrapDockerReport;
use crate::runner::{CommandRunner, IoMode};
use std::fs;

pub fn execute_plan(
    runner: &dyn CommandRunner,
    config: &DockerConfig,
    operations: Vec<DockerPlannedOperation>,
    io_mode: IoMode,
) -> anyhow::Result<BootstrapDockerReport> {
    let mut results = Vec::new();

    for operation in operations {
        match &operation {
            DockerPlannedOperation::Subprocess {
                id,
                description,
                command,
                args,
                env,
                failure_is_warning,
            } => {
                if config.dry_run {
                    let detail = format_dry_run_detail(command, args, env);
                    results.push(OperationResult {
                        id,
                        description,
                        status: OperationStatus::Planned,
                        detail,
                    });
                    continue;
                }

                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let env_refs: Vec<(&str, &str)> =
                    env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

                let result = runner.run_with_env_io(command, &args_refs, &env_refs, io_mode);

                match result {
                    Ok(output) if output.status.success() => {
                        results.push(OperationResult {
                            id,
                            description,
                            status: OperationStatus::Succeeded,
                            detail: String::new(),
                        });
                    }
                    Ok(output) if *failure_is_warning => {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        results.push(OperationResult {
                            id,
                            description,
                            status: OperationStatus::Skipped,
                            detail: format!("warning (non-fatal): {}", stderr),
                        });
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        results.push(OperationResult {
                            id,
                            description,
                            status: OperationStatus::Failed,
                            detail: stderr,
                        });
                        break;
                    }
                    Err(e) if *failure_is_warning => {
                        results.push(OperationResult {
                            id,
                            description,
                            status: OperationStatus::Skipped,
                            detail: format!("warning (non-fatal): {}", e),
                        });
                    }
                    Err(e) => {
                        results.push(OperationResult {
                            id,
                            description,
                            status: OperationStatus::Failed,
                            detail: e.to_string(),
                        });
                        break;
                    }
                }
            }
            DockerPlannedOperation::WriteFile {
                id,
                description,
                path,
                content,
            } => {
                if config.dry_run {
                    results.push(OperationResult {
                        id,
                        description,
                        status: OperationStatus::Planned,
                        detail: format!("dry-run: write {}", path.display()),
                    });
                    continue;
                }

                match fs::write(path, content) {
                    Ok(_) => {
                        results.push(OperationResult {
                            id,
                            description,
                            status: OperationStatus::Succeeded,
                            detail: String::new(),
                        });
                    }
                    Err(e) => {
                        results.push(OperationResult {
                            id,
                            description,
                            status: OperationStatus::Failed,
                            detail: e.to_string(),
                        });
                        break;
                    }
                }
            }
        }
    }

    Ok(BootstrapDockerReport {
        operations: results,
    })
}

fn format_dry_run_detail(command: &str, args: &[String], env: &[(String, String)]) -> String {
    let mut detail = String::new();

    if !env.is_empty() {
        let env_str = env
            .iter()
            .map(|(k, v)| {
                let is_sensitive =
                    k.contains("TOKEN") || k.contains("SECRET") || k.contains("PASSWORD");
                if is_sensitive {
                    format!("{}=<redacted>", k)
                } else {
                    format!("{}={}", k, v)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        detail.push_str(&format!("env=[{}] ", env_str));
    }

    detail.push_str("dry-run: ");
    detail.push_str(&format!("{} {}", command, args.join(" ")));
    detail
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_dry_run_detail_redacts_token() {
        let env = vec![("REGISTRY_TOKEN".to_string(), "secret123".to_string())];
        let detail = format_dry_run_detail("docker", &["login".to_string()], &env);
        assert!(detail.contains("REGISTRY_TOKEN=<redacted>"));
        assert!(!detail.contains("secret123"));
    }

    #[test]
    fn format_dry_run_detail_redacts_secret() {
        let env = vec![("MY_SECRET".to_string(), "secret123".to_string())];
        let detail = format_dry_run_detail("docker", &["run".to_string()], &env);
        assert!(detail.contains("MY_SECRET=<redacted>"));
    }

    #[test]
    fn format_dry_run_detail_redacts_password() {
        let env = vec![("REGISTRY_PASSWORD".to_string(), "pass123".to_string())];
        let detail = format_dry_run_detail("docker", &["login".to_string()], &env);
        assert!(detail.contains("REGISTRY_PASSWORD=<redacted>"));
    }

    #[test]
    fn format_dry_run_detail_keeps_non_sensitive() {
        let env = vec![("INSTALL_K3S_VERSION".to_string(), "v1.28.0".to_string())];
        let detail = format_dry_run_detail("sh", &["-c".to_string()], &env);
        assert!(detail.contains("INSTALL_K3S_VERSION=v1.28.0"));
    }
}
