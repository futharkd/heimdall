use anyhow::{Result, bail};

use super::input::BootstrapFluxConfig;

pub const FLUX_SYNC_NAME: &str = "flux-system";

#[derive(Debug, Clone)]
pub struct FluxPlannedOperation {
    pub id: &'static str,
    pub description: &'static str,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub failure_is_warning: bool,
}

pub fn build_plan(config: &BootstrapFluxConfig) -> Result<Vec<FluxPlannedOperation>> {
    let kube_env = super::input::kube_env(&config.kubeconfig);

    if config.namespace_exists {
        return Ok(reconcile_plan(config, kube_env.as_slice()));
    }

    let mut ops = Vec::new();

    if !config.skip_flux_cli_install {
        let install_path = config
            .install_script_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("install script path is not valid UTF-8"))?
            .to_string();

        ops.push(FluxPlannedOperation {
            id: "download_flux_install_script",
            description: "Download official Flux install script",
            command: "curl".to_string(),
            args: vec![
                "-fsSL".to_string(),
                "-o".to_string(),
                install_path.clone(),
                config.install_script_url.clone(),
            ],
            env: vec![],
            failure_is_warning: false,
        });

        ops.push(FluxPlannedOperation {
            id: "run_flux_install_script",
            description: "Execute Flux install script (installs `flux` CLI)",
            command: "bash".to_string(),
            args: vec![install_path],
            env: vec![],
            failure_is_warning: false,
        });
    }

    let Some(ref pk) = config.private_key_bootstrap_path else {
        bail!("internal error: missing private key path for Flux bootstrap");
    };
    let pk_str = pk
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("private key path is not valid UTF-8"))?
        .to_string();

    let git_url = super::validate::normalize_ssh_git_url_for_flux(&config.git_url);
    let mut bootstrap_args = vec![
        "bootstrap".to_string(),
        "git".to_string(),
        format!("--url={git_url}"),
        format!("--private-key-file={pk_str}"),
        format!("--branch={}", config.branch),
        format!("--path={}", config.cluster_path),
        format!("--namespace={}", config.namespace),
        format!("--kubeconfig={}", config.kubeconfig),
        "--silent".to_string(),
    ];
    if let Some(ref pass) = config.private_key_passphrase {
        bootstrap_args.push("--password".to_string());
        bootstrap_args.push(pass.clone());
    }

    ops.push(FluxPlannedOperation {
        id: "flux_bootstrap_git",
        description: "Flux bootstrap git (SSH deploy key; commits manifests and configures cluster)",
        command: "flux".to_string(),
        args: bootstrap_args,
        env: kube_env.clone(),
        failure_is_warning: false,
    });

    ops.push(FluxPlannedOperation {
        id: "flux_get_kustomization",
        description: "Verify Flux kustomization is known to the API",
        command: "flux".to_string(),
        args: vec![
            "get".to_string(),
            "kustomization".to_string(),
            FLUX_SYNC_NAME.to_string(),
            "-n".to_string(),
            config.namespace.clone(),
            "--kubeconfig".to_string(),
            config.kubeconfig.clone(),
        ],
        env: kube_env,
        failure_is_warning: false,
    });

    Ok(ops)
}

fn reconcile_plan(
    config: &BootstrapFluxConfig,
    kube_env: &[(String, String)],
) -> Vec<FluxPlannedOperation> {
    let ns = config.namespace.clone();
    let kc = config.kubeconfig.clone();
    vec![
        FluxPlannedOperation {
            id: "flux_reconcile_source_git",
            description: "Reconcile existing Flux Git source",
            command: "flux".to_string(),
            args: vec![
                "reconcile".to_string(),
                "source".to_string(),
                "git".to_string(),
                FLUX_SYNC_NAME.to_string(),
                "-n".to_string(),
                ns.clone(),
                "--kubeconfig".to_string(),
                kc.clone(),
            ],
            env: kube_env.to_vec(),
            failure_is_warning: false,
        },
        FluxPlannedOperation {
            id: "flux_reconcile_kustomization",
            description: "Reconcile existing Flux kustomization",
            command: "flux".to_string(),
            args: vec![
                "reconcile".to_string(),
                "kustomization".to_string(),
                FLUX_SYNC_NAME.to_string(),
                "-n".to_string(),
                ns.clone(),
                "--kubeconfig".to_string(),
                kc.clone(),
            ],
            env: kube_env.to_vec(),
            failure_is_warning: false,
        },
        FluxPlannedOperation {
            id: "flux_get_kustomization",
            description: "Verify Flux kustomization after reconcile",
            command: "flux".to_string(),
            args: vec![
                "get".to_string(),
                "kustomization".to_string(),
                FLUX_SYNC_NAME.to_string(),
                "-n".to_string(),
                ns,
                "--kubeconfig".to_string(),
                kc,
            ],
            env: kube_env.to_vec(),
            failure_is_warning: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{FLUX_SYNC_NAME, build_plan};
    use crate::features::bootstrap::flux::input::BootstrapFluxConfig;

    fn base_config() -> BootstrapFluxConfig {
        BootstrapFluxConfig {
            install_script_path: PathBuf::from("/tmp/flux-install.sh"),
            install_script_url: "https://fluxcd.io/install.sh".to_string(),
            git_url: "ssh://git@gitlab.com/g/r.git".to_string(),
            branch: "main".to_string(),
            cluster_path: "clusters/x".to_string(),
            namespace: "flux-system".to_string(),
            kubeconfig: "/kube".to_string(),
            dry_run: false,
            force: false,
            skip_flux_cli_install: false,
            namespace_exists: false,
            byok_private_key: None,
            private_key_bootstrap_path: Some(PathBuf::from("/tmp/priv")),
            ephemeral_key_pair_root: None,
            ephemeral_key_generated: false,
            private_key_passphrase: None,
            keep_generated_key_dir: None,
        }
    }

    #[test]
    fn bootstrap_plan_includes_git_and_kubeconfig() {
        let plan = build_plan(&base_config()).expect("plan");
        let bootstrap = plan
            .iter()
            .find(|o| o.id == "flux_bootstrap_git")
            .expect("bootstrap");
        assert!(bootstrap.args.iter().any(|a| a.starts_with("--url=ssh://")));
        assert!(
            bootstrap
                .args
                .iter()
                .any(|a| a.starts_with("--private-key-file="))
        );
        assert!(!bootstrap.args.iter().any(|a| a == "--token-auth"));
        assert!(bootstrap.args.contains(&"--silent".to_string()));
        assert!(bootstrap.env.iter().any(|(k, _)| k == "KUBECONFIG"));
    }

    #[test]
    fn reconcile_plan_targets_flux_system() {
        let mut c = base_config();
        c.namespace_exists = true;
        let plan = build_plan(&c).expect("plan");
        assert_eq!(plan.len(), 3);
        assert!(plan.iter().any(|o| o.id == "flux_reconcile_source_git"));
        let src = plan.first().expect("src");
        assert!(src.args.contains(&FLUX_SYNC_NAME.to_string()));
    }

    #[test]
    fn skip_flux_install_omits_curl_bash() {
        let mut c = base_config();
        c.skip_flux_cli_install = true;
        let plan = build_plan(&c).expect("plan");
        assert!(!plan.iter().any(|o| o.id == "download_flux_install_script"));
        assert_eq!(plan.first().map(|o| o.id), Some("flux_bootstrap_git"));
    }

    #[test]
    fn bootstrap_plan_normalizes_scp_git_url_for_flux_cli() {
        let mut c = base_config();
        c.git_url = "git@gitlab.com:futharkd/cluster.git".to_string();
        let plan = build_plan(&c).expect("plan");
        let bootstrap = plan
            .iter()
            .find(|o| o.id == "flux_bootstrap_git")
            .expect("bootstrap");
        assert!(
            bootstrap
                .args
                .iter()
                .any(|a| { a.as_str() == "--url=ssh://git@gitlab.com/futharkd/cluster.git" })
        );
    }
}
