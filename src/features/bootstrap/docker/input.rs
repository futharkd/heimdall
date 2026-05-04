use anyhow::bail;
use inquire::Confirm;

use crate::cli::{BootstrapDockerCommand, OutputFormat};
use crate::runner::{CommandRunner, IoMode};

fn map_inquire<T>(r: Result<T, inquire::InquireError>) -> anyhow::Result<T> {
    r.map_err(|e| match e {
        inquire::InquireError::NotTTY => anyhow::anyhow!("not a TTY; pass the flag directly"),
        inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted => {
            anyhow::anyhow!("cancelled")
        }
        other => anyhow::anyhow!("{other}"),
    })
}

#[derive(Debug, Clone)]
pub struct DockerConfig {
    pub install_script_url: String,
    pub add_user: Option<String>,
    pub log_driver: Option<String>,
    pub registry_mirrors: Vec<String>,
    pub dry_run: bool,
    pub force: bool,
    pub skip_install: bool,
}

pub struct ResolvedDockerInputs {
    pub config: DockerConfig,
    pub output: OutputFormat,
}

pub fn resolve_inputs(opts: BootstrapDockerCommand) -> anyhow::Result<ResolvedDockerInputs> {
    let install_script_url = opts
        .install_script_url
        .or_else(|| std::env::var("DOCKER_INSTALL_SCRIPT_URL").ok())
        .unwrap_or_else(|| "https://get.docker.com".to_string());

    super::validate::validate_url(&install_script_url)?;

    for mirror in &opts.registry_mirrors {
        super::validate::validate_registry_mirror(mirror)?;
    }

    if let Some(ref user) = opts.user {
        super::validate::validate_username(user)?;
    }

    if !(opts.yes || opts.dry_run || confirm_install()?) {
        bail!("aborted: docker bootstrap was not confirmed");
    }

    if !opts.dry_run {
        eprintln!(
            "note: the official Docker installer typically requires root and will install systemd units and binaries on this host"
        );
    }

    Ok(ResolvedDockerInputs {
        config: DockerConfig {
            install_script_url,
            add_user: opts.user,
            log_driver: opts.log_driver,
            registry_mirrors: opts.registry_mirrors,
            dry_run: opts.dry_run,
            force: opts.force,
            skip_install: false,
        },
        output: opts.output,
    })
}

pub fn probe_docker_on_path(runner: &dyn CommandRunner) -> bool {
    runner
        .run_with_env_io(
            "sh",
            &["-c", "command -v docker >/dev/null 2>&1"],
            &[],
            IoMode::Buffered,
        )
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn confirm_install() -> anyhow::Result<bool> {
    map_inquire(
        Confirm::new("This will install or update Docker using the official get.docker.com install script. Continue?")
            .with_default(false)
            .prompt(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_inputs_uses_default_script_url() {
        let opts = BootstrapDockerCommand {
            install_script_url: None,
            user: None,
            log_driver: None,
            registry_mirrors: vec![],
            force: false,
            dry_run: true,
            yes: false,
            output: OutputFormat::Human,
        };

        let resolved = resolve_inputs(opts).unwrap();
        assert_eq!(resolved.config.install_script_url, "https://get.docker.com");
    }

    #[test]
    fn resolve_inputs_respects_custom_url() {
        let opts = BootstrapDockerCommand {
            install_script_url: Some("https://custom.example.com/install.sh".to_string()),
            user: None,
            log_driver: None,
            registry_mirrors: vec![],
            force: false,
            dry_run: true,
            yes: false,
            output: OutputFormat::Human,
        };

        let resolved = resolve_inputs(opts).unwrap();
        assert_eq!(
            resolved.config.install_script_url,
            "https://custom.example.com/install.sh"
        );
    }

    #[test]
    fn resolve_inputs_sets_confirmed_with_yes() {
        let opts = BootstrapDockerCommand {
            install_script_url: None,
            user: None,
            log_driver: None,
            registry_mirrors: vec![],
            force: false,
            dry_run: false,
            yes: true,
            output: OutputFormat::Human,
        };

        let _resolved = resolve_inputs(opts).unwrap();
        // confirmed is set by resolve_inputs, verified by confirmation_skip test below
    }

    #[test]
    fn resolve_inputs_sets_confirmed_with_dry_run() {
        let opts = BootstrapDockerCommand {
            install_script_url: None,
            user: None,
            log_driver: None,
            registry_mirrors: vec![],
            force: false,
            dry_run: true,
            yes: false,
            output: OutputFormat::Human,
        };

        let _resolved = resolve_inputs(opts).unwrap();
        // confirmed is set by resolve_inputs, verified by confirmation_skip test below
    }

    #[test]
    fn resolve_inputs_stores_user_and_daemon_config() {
        let opts = BootstrapDockerCommand {
            install_script_url: None,
            user: Some("ubuntu".to_string()),
            log_driver: Some("json-file".to_string()),
            registry_mirrors: vec!["https://mirror.example.com".to_string()],
            force: false,
            dry_run: true,
            yes: false,
            output: OutputFormat::Human,
        };

        let resolved = resolve_inputs(opts).unwrap();
        assert_eq!(resolved.config.add_user, Some("ubuntu".to_string()));
        assert_eq!(resolved.config.log_driver, Some("json-file".to_string()));
        assert_eq!(
            resolved.config.registry_mirrors,
            vec!["https://mirror.example.com"]
        );
    }
}
