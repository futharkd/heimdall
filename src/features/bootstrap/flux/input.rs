use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::cli::{BootstrapFluxCommand, OutputFormat};
use crate::runner::{CommandRunner, IoMode};

#[derive(Debug, Clone)]
pub struct BootstrapFluxConfig {
    pub install_script_path: PathBuf,
    pub install_script_url: String,
    pub git_url: String,
    pub branch: String,
    pub cluster_path: String,
    pub namespace: String,
    pub kubeconfig: String,
    pub dry_run: bool,
    pub force: bool,
    pub skip_flux_cli_install: bool,
    /// Set in `command::run` after `kubectl get ns` probe.
    pub namespace_exists: bool,
    /// User-supplied key path (BYOK); never written by Heimdall.
    pub byok_private_key: Option<PathBuf>,
    /// Path passed to `flux bootstrap git --private-key-file`.
    pub private_key_bootstrap_path: Option<PathBuf>,
    /// Private key path when Heimdall generated it (same file ssh-keygen wrote); cleanup deletes this + `.pub`.
    pub ephemeral_key_pair_root: Option<PathBuf>,
    pub ephemeral_key_generated: bool,
    pub private_key_passphrase: Option<String>,
    pub keep_generated_key_dir: Option<PathBuf>,
}

pub struct ResolvedFluxInputs {
    pub config: BootstrapFluxConfig,
    pub output: OutputFormat,
}

/// Flux manifest path from `--path` or `FLUX_GIT_PATH` when set (trimmed, non-empty).
pub(crate) fn cluster_path_from_opts_and_env(opts: &BootstrapFluxCommand) -> Option<String> {
    opts.path
        .clone()
        .or_else(|| std::env::var("FLUX_GIT_PATH").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Git URL from `--url` or `FLUX_GIT_URL` when set (trimmed, non-empty).
pub(crate) fn git_url_from_opts_and_env(opts: &BootstrapFluxCommand) -> Option<String> {
    opts.url
        .clone()
        .or_else(|| std::env::var("FLUX_GIT_URL").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn resolve_git_url(opts: &BootstrapFluxCommand) -> Result<String> {
    if let Some(url) = git_url_from_opts_and_env(opts) {
        super::validate::validate_ssh_git_url(&url)?;
        return Ok(url);
    }
    if !io::stdin().is_terminal() {
        bail!(
            "Git URL not set: pass --url, set FLUX_GIT_URL, or run from a terminal for an interactive prompt"
        );
    }
    loop {
        let line = prompt("Git SSH clone URL (e.g. ssh://git@gitlab.com/group/repo.git): ")?;
        let t = line.trim();
        if t.is_empty() {
            eprintln!("A non-empty SSH Git URL is required.");
            continue;
        }
        match super::validate::validate_ssh_git_url(t) {
            Ok(()) => return Ok(t.to_string()),
            Err(e) => eprintln!("{e}"),
        }
    }
}

fn resolve_cluster_path(opts: &BootstrapFluxCommand) -> Result<String> {
    if let Some(p) = cluster_path_from_opts_and_env(opts) {
        return Ok(p);
    }
    if !io::stdin().is_terminal() {
        bail!(
            "Flux path in repo not set: pass --path, set FLUX_GIT_PATH, or run from a terminal for an interactive prompt"
        );
    }
    loop {
        let line = prompt(
            "Path inside the Git repo for Flux manifests (e.g. clusters/prod): ",
        )?;
        let t = line.trim();
        if t.is_empty() {
            eprintln!("A non-empty path is required.");
            continue;
        }
        return Ok(t.to_string());
    }
}

pub fn resolve_inputs(opts: BootstrapFluxCommand) -> Result<ResolvedFluxInputs> {
    let install_script_path = std::env::temp_dir().join(format!(
        "heimdall-flux-install-{}.sh",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    let git_url = resolve_git_url(&opts)?;

    let branch = opts
        .branch
        .clone()
        .or_else(|| std::env::var("FLUX_GIT_BRANCH").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "main".to_string());

    let cluster_path = resolve_cluster_path(&opts)?;

    let namespace = opts
        .namespace
        .clone()
        .or_else(|| std::env::var("FLUX_NAMESPACE").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "flux-system".to_string());

    let kubeconfig = opts
        .kubeconfig
        .clone()
        .or_else(|| std::env::var("KUBECONFIG").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/etc/rancher/k3s/k3s.yaml".to_string());

    let install_script_url = opts
        .install_script_url
        .clone()
        .unwrap_or_else(|| "https://fluxcd.io/install.sh".to_string());

    let byok_private_key = opts
        .private_key_file
        .as_ref()
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty());

    if let Some(ref p) = byok_private_key
        && !p.is_file()
    {
        bail!("--private-key-file must point to an existing file: {p:?}");
    }

    let private_key_passphrase = opts
        .private_key_passphrase
        .clone()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if private_key_passphrase.is_some() && byok_private_key.is_none() {
        bail!("--private-key-passphrase is only valid with --private-key-file");
    }

    let keep_generated_key_dir = opts
        .keep_generated_key
        .as_ref()
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty());

    if !(opts.yes || opts.dry_run || confirm_flux_bootstrap()?) {
        bail!("aborted: Flux bootstrap was not confirmed");
    }

    Ok(ResolvedFluxInputs {
        config: BootstrapFluxConfig {
            install_script_path,
            install_script_url,
            git_url,
            branch,
            cluster_path,
            namespace,
            kubeconfig,
            dry_run: opts.dry_run,
            force: opts.force,
            skip_flux_cli_install: false,
            namespace_exists: false,
            byok_private_key,
            private_key_bootstrap_path: None,
            ephemeral_key_pair_root: None,
            ephemeral_key_generated: false,
            private_key_passphrase,
            keep_generated_key_dir,
        },
        output: opts.output,
    })
}

pub fn kube_env(kubeconfig: &str) -> Vec<(String, String)> {
    vec![("KUBECONFIG".to_string(), kubeconfig.to_string())]
}

fn confirm_flux_bootstrap() -> Result<bool> {
    let answer = prompt(
        "This will install or configure Flux against your cluster and Git repo. Continue? type 'yes' to proceed: ",
    )?;
    Ok(answer == "yes")
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

/// Returns `true` if `kubectl get ns <namespace>` succeeds.
pub fn probe_flux_namespace(
    runner: &dyn CommandRunner,
    kubeconfig: &str,
    namespace: &str,
) -> Result<bool> {
    let env = kube_env(kubeconfig);
    let env_refs: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let args = ["get", "ns", namespace];
    let output = runner.run_with_env_io("kubectl", &args, &env_refs, IoMode::Buffered)?;
    Ok(output.status.success())
}

pub fn probe_flux_on_path(runner: &dyn CommandRunner) -> bool {
    runner
        .run_with_env_io("flux", &["version"], &[], IoMode::Buffered)
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn wait_enter_after_deploy_key_prompt() -> Result<()> {
    println!();
    print!(
        "Press Enter after you saved the deploy key on GitLab or GitHub (write access required for bootstrap)… "
    );
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{cluster_path_from_opts_and_env, git_url_from_opts_and_env};
    use crate::cli::{BootstrapFluxCommand, OutputFormat};

    fn flux_cmd(url: Option<&str>, path: Option<&str>) -> BootstrapFluxCommand {
        BootstrapFluxCommand {
            url: url.map(String::from),
            branch: None,
            path: path.map(String::from),
            namespace: None,
            kubeconfig: None,
            private_key_file: None,
            private_key_passphrase: None,
            install_script_url: None,
            keep_generated_key: None,
            force: false,
            dry_run: false,
            yes: true,
            output: OutputFormat::Human,
        }
    }

    #[test]
    fn git_url_from_opts_trims_flag() {
        assert_eq!(
            git_url_from_opts_and_env(&flux_cmd(Some("  ssh://git@x/y.git  "), None)).as_deref(),
            Some("ssh://git@x/y.git")
        );
    }

    #[test]
    fn git_url_from_opts_none_when_missing() {
        assert!(git_url_from_opts_and_env(&flux_cmd(None, None)).is_none());
    }

    #[test]
    fn cluster_path_from_opts_trims_flag() {
        assert_eq!(
            cluster_path_from_opts_and_env(&flux_cmd(
                None,
                Some("  clusters/prod  ")
            ))
            .as_deref(),
            Some("clusters/prod")
        );
    }

    #[test]
    fn cluster_path_from_opts_none_when_missing() {
        assert!(cluster_path_from_opts_and_env(&flux_cmd(None, None)).is_none());
    }
}
