use std::io::{self, ErrorKind, IsTerminal};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Result, bail};
use inquire::Text;

use crate::cli::{BootstrapFluxCommand, OutputFormat};
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
    /// When true, `kubectl` / `flux` that touch the API run under `sudo` (root-only kubeconfig).
    pub kube_elevated: bool,
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

/// Branch from `--branch` or `FLUX_GIT_BRANCH` when set (trimmed, non-empty).
pub(crate) fn branch_from_opts_and_env(opts: &BootstrapFluxCommand) -> Option<String> {
    opts.branch
        .clone()
        .or_else(|| std::env::var("FLUX_GIT_BRANCH").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_default_branch_from_ls_remote(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        let (lhs, rhs) = line.split_once('\t')?;
        if rhs != "HEAD" {
            return None;
        }
        lhs.strip_prefix("ref: refs/heads/")
            .map(ToString::to_string)
    })
}

fn detect_remote_default_branch(git_url: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["ls-remote", "--symref", git_url, "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_default_branch_from_ls_remote(&stdout)
}

fn resolve_branch_with<F>(
    opts: &BootstrapFluxCommand,
    git_url: &str,
    stdin_is_tty: bool,
    detect_default_branch: F,
) -> Result<String>
where
    F: FnOnce(&str) -> Option<String>,
{
    if let Some(branch) = branch_from_opts_and_env(opts) {
        return Ok(branch);
    }

    if let Some(branch) = detect_default_branch(git_url) {
        return Ok(branch);
    }

    if !stdin_is_tty {
        bail!(
            "Flux branch not set and remote default branch could not be detected; pass --branch, set FLUX_GIT_BRANCH, or run from a terminal for an interactive prompt"
        );
    }

    let entered = map_inquire(
        Text::new("Git branch for Flux bootstrap:")
            .with_default("main")
            .prompt(),
    )?;
    let trimmed = entered.trim();
    if trimmed.is_empty() {
        Ok("main".to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn resolve_branch(opts: &BootstrapFluxCommand, git_url: &str) -> Result<String> {
    resolve_branch_with(
        opts,
        git_url,
        io::stdin().is_terminal(),
        detect_remote_default_branch,
    )
}

fn resolve_git_url(opts: &BootstrapFluxCommand) -> Result<String> {
    if let Some(url) = git_url_from_opts_and_env(opts) {
        return super::validate::finalize_flux_git_url(&url);
    }
    loop {
        let line = map_inquire(
            Text::new("Git SSH clone URL (e.g. ssh://git@gitlab.com/group/repo.git or git@gitlab.com:group/repo.git):")
                .prompt(),
        )?;
        let t = line.trim();
        if t.is_empty() {
            eprintln!("A non-empty SSH Git URL is required.");
            continue;
        }
        match super::validate::finalize_flux_git_url(t) {
            Ok(url) => return Ok(url),
            Err(e) => eprintln!("{e}"),
        }
    }
}

fn resolve_cluster_path(opts: &BootstrapFluxCommand) -> Result<String> {
    if let Some(p) = cluster_path_from_opts_and_env(opts) {
        return Ok(p);
    }
    loop {
        let line = map_inquire(
            Text::new("Path inside the Git repo for Flux manifests (e.g. clusters/prod):").prompt(),
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

    let branch = resolve_branch(&opts, &git_url)?;

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
            kube_elevated: false,
        },
        output: opts.output,
    })
}

/// True when the kubeconfig cannot be read as the current user (typical for `/etc/rancher/k3s/k3s.yaml`).
pub fn kubeconfig_requires_elevated_access(path: &str) -> bool {
    match std::fs::File::open(path) {
        Ok(_) => false,
        Err(e) => e.kind() == ErrorKind::PermissionDenied,
    }
}

pub fn kube_env(kubeconfig: &str) -> Vec<(String, String)> {
    vec![("KUBECONFIG".to_string(), kubeconfig.to_string())]
}

fn confirm_flux_bootstrap() -> Result<bool> {
    use inquire::Confirm;
    map_inquire(
        Confirm::new("Configure Flux against your cluster and Git repo. Continue?")
            .with_default(false)
            .prompt(),
    )
}

/// Returns `true` if `kubectl get ns <namespace>` succeeds.
pub fn probe_flux_namespace(
    runner: &dyn CommandRunner,
    kubeconfig: &str,
    namespace: &str,
    kube_elevated: bool,
) -> Result<bool> {
    let env = kube_env(kubeconfig);
    let env_refs: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let output = if kube_elevated {
        let args = ["k3s", "kubectl", "get", "ns", namespace];
        runner.run_with_env_io("sudo", &args, &env_refs, IoMode::Buffered)?
    } else {
        let args = ["get", "ns", namespace];
        runner.run_with_env_io("kubectl", &args, &env_refs, IoMode::Buffered)?
    };
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
    map_inquire(
        Text::new("")
            .with_help_message("Press Enter after you saved the deploy key (with write access — bootstrap must push initial commits to the repo)…")
            .prompt(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        branch_from_opts_and_env, cluster_path_from_opts_and_env, git_url_from_opts_and_env,
        kubeconfig_requires_elevated_access, parse_default_branch_from_ls_remote,
        resolve_branch_with,
    };
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
    fn branch_from_opts_trims_flag() {
        let mut cmd = flux_cmd(None, None);
        cmd.branch = Some("  master  ".to_string());
        assert_eq!(branch_from_opts_and_env(&cmd).as_deref(), Some("master"));
    }

    #[test]
    fn parse_default_branch_from_symref_output() {
        let stdout = "ref: refs/heads/master\tHEAD\n0123456789abcdef\tHEAD\n";
        assert_eq!(
            parse_default_branch_from_ls_remote(stdout).as_deref(),
            Some("master")
        );
    }

    #[test]
    fn resolve_branch_prefers_explicit_over_detection() {
        let mut cmd = flux_cmd(None, None);
        cmd.branch = Some("main".to_string());
        let branch =
            resolve_branch_with(&cmd, "ssh://git@x/y.git", false, |_| Some("master".into()))
                .expect("branch");
        assert_eq!(branch, "main");
    }

    #[test]
    fn resolve_branch_uses_detected_remote_default() {
        let cmd = flux_cmd(None, None);
        let branch =
            resolve_branch_with(&cmd, "ssh://git@x/y.git", false, |_| Some("master".into()))
                .expect("branch");
        assert_eq!(branch, "master");
    }

    #[test]
    fn resolve_branch_non_interactive_fails_when_undetected() {
        let cmd = flux_cmd(None, None);
        let err =
            resolve_branch_with(&cmd, "ssh://git@x/y.git", false, |_| None).expect_err("must fail");
        let msg = err.to_string();
        assert!(msg.contains("Flux branch not set"));
        assert!(msg.contains("--branch"));
        assert!(msg.contains("FLUX_GIT_BRANCH"));
    }

    #[test]
    fn cluster_path_from_opts_trims_flag() {
        assert_eq!(
            cluster_path_from_opts_and_env(&flux_cmd(None, Some("  clusters/prod  "))).as_deref(),
            Some("clusters/prod")
        );
    }

    #[test]
    fn cluster_path_from_opts_none_when_missing() {
        assert!(cluster_path_from_opts_and_env(&flux_cmd(None, None)).is_none());
    }

    #[test]
    fn kubeconfig_readable_does_not_require_elevated_access() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("kc.yaml");
        std::fs::write(&path, b"apiVersion: v1\n").expect("write");
        assert!(!kubeconfig_requires_elevated_access(
            path.to_str().expect("utf8")
        ));
    }

    #[cfg(unix)]
    fn unix_euid_is_root() -> bool {
        #[link(name = "c")]
        unsafe extern "C" {
            fn geteuid() -> u32;
        }
        unsafe { geteuid() == 0 }
    }

    /// Root ignores mode bits for read; GitLab CI often runs the job as root, so chmod 000 does not
    /// produce `PermissionDenied` for the test process.
    #[cfg(unix)]
    #[test]
    fn kubeconfig_requires_elevated_when_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        if unix_euid_is_root() {
            return;
        }

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("kc.yaml");
        std::fs::write(&path, b"apiVersion: v1\n").expect("write");
        let mut perms = std::fs::metadata(&path).expect("meta").permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&path, perms).expect("chmod");
        assert!(kubeconfig_requires_elevated_access(
            path.to_str().expect("utf8")
        ));
    }
}
