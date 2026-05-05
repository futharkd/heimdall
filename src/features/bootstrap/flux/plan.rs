use anyhow::{Result, bail};

use super::input::BootstrapFluxConfig;
use crate::core::operation::{OperationKind, PlannedOperation};

pub const FLUX_SYNC_NAME: &str = "flux-system";

fn sudo_wrap_flux(args: Vec<String>, kube_elevated: bool) -> (String, Vec<String>) {
    if kube_elevated {
        let mut prefixed = vec!["flux".to_string()];
        prefixed.extend(args);
        ("sudo".to_string(), prefixed)
    } else {
        ("flux".to_string(), args)
    }
}

pub fn build_plan(config: &BootstrapFluxConfig) -> Result<Vec<PlannedOperation>> {
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

        ops.push(PlannedOperation {
            id: "download_flux_install_script",
            description: "Download official Flux install script".to_string(),
            kind: OperationKind::Shell {
                command: "curl".to_string(),
                args: vec![
                    "-fsSL".to_string(),
                    "-o".to_string(),
                    install_path.clone(),
                    config.install_script_url.clone(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        });

        ops.push(PlannedOperation {
            id: "run_flux_install_script",
            description: "Execute Flux install script (installs `flux` CLI)".to_string(),
            kind: OperationKind::Shell {
                command: "bash".to_string(),
                args: vec![install_path],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
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

    let (bootstrap_cmd, bootstrap_cmd_args) = sudo_wrap_flux(bootstrap_args, config.kube_elevated);

    ops.push(PlannedOperation {
        id: "flux_bootstrap_git",
        description:
            "Flux bootstrap git (SSH deploy key; commits manifests and configures cluster)"
                .to_string(),
        kind: OperationKind::Shell {
            command: bootstrap_cmd,
            args: bootstrap_cmd_args,
            env: kube_env.clone(),
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    let get_args = vec![
        "get".to_string(),
        "kustomization".to_string(),
        FLUX_SYNC_NAME.to_string(),
        "-n".to_string(),
        config.namespace.clone(),
        "--kubeconfig".to_string(),
        config.kubeconfig.clone(),
    ];
    let (get_cmd, get_cmd_args) = sudo_wrap_flux(get_args, config.kube_elevated);

    ops.push(PlannedOperation {
        id: "flux_get_kustomization",
        description: "Verify Flux kustomization is known to the API".to_string(),
        kind: OperationKind::Shell {
            command: get_cmd,
            args: get_cmd_args,
            env: kube_env,
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    Ok(ops)
}

fn reconcile_plan(
    config: &BootstrapFluxConfig,
    kube_env: &[(String, String)],
) -> Vec<PlannedOperation> {
    let ns = config.namespace.clone();
    let kc = config.kubeconfig.clone();
    let elevate = config.kube_elevated;

    let (src_cmd, src_args) = sudo_wrap_flux(
        vec![
            "reconcile".to_string(),
            "source".to_string(),
            "git".to_string(),
            FLUX_SYNC_NAME.to_string(),
            "-n".to_string(),
            ns.clone(),
            "--kubeconfig".to_string(),
            kc.clone(),
        ],
        elevate,
    );

    let (kust_cmd, kust_args) = sudo_wrap_flux(
        vec![
            "reconcile".to_string(),
            "kustomization".to_string(),
            FLUX_SYNC_NAME.to_string(),
            "-n".to_string(),
            ns.clone(),
            "--kubeconfig".to_string(),
            kc.clone(),
        ],
        elevate,
    );

    let (get_cmd, get_args) = sudo_wrap_flux(
        vec![
            "get".to_string(),
            "kustomization".to_string(),
            FLUX_SYNC_NAME.to_string(),
            "-n".to_string(),
            ns,
            "--kubeconfig".to_string(),
            kc,
        ],
        elevate,
    );

    vec![
        PlannedOperation {
            id: "flux_reconcile_source_git",
            description: "Reconcile existing Flux Git source".to_string(),
            kind: OperationKind::Shell {
                command: src_cmd,
                args: src_args,
                env: kube_env.to_vec(),
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        },
        PlannedOperation {
            id: "flux_reconcile_kustomization",
            description: "Reconcile existing Flux kustomization".to_string(),
            kind: OperationKind::Shell {
                command: kust_cmd,
                args: kust_args,
                env: kube_env.to_vec(),
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        },
        PlannedOperation {
            id: "flux_get_kustomization",
            description: "Verify Flux kustomization after reconcile".to_string(),
            kind: OperationKind::Shell {
                command: get_cmd,
                args: get_args,
                env: kube_env.to_vec(),
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
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
            kube_elevated: false,
        }
    }

    #[test]
    fn bootstrap_plan_includes_git_and_kubeconfig() {
        use crate::core::operation::OperationKind;
        let plan = build_plan(&base_config()).expect("plan");
        let bootstrap = plan
            .iter()
            .find(|o| o.id == "flux_bootstrap_git")
            .expect("bootstrap");
        if let OperationKind::Shell { args, env, .. } = &bootstrap.kind {
            assert!(args.iter().any(|a| a.starts_with("--url=ssh://")));
            assert!(args.iter().any(|a| a.starts_with("--private-key-file=")));
            assert!(!args.iter().any(|a| a == "--token-auth"));
            assert!(args.contains(&"--silent".to_string()));
            assert!(env.iter().any(|(k, _)| k == "KUBECONFIG"));
        } else {
            panic!("expected Shell kind");
        }
    }

    #[test]
    fn bootstrap_plan_wraps_flux_in_sudo_when_kube_elevated() {
        use crate::core::operation::OperationKind;
        let mut c = base_config();
        c.kube_elevated = true;
        let plan = build_plan(&c).expect("plan");
        let bootstrap = plan
            .iter()
            .find(|o| o.id == "flux_bootstrap_git")
            .expect("bootstrap");
        if let OperationKind::Shell { command, args, .. } = &bootstrap.kind {
            assert_eq!(command, "sudo");
            assert_eq!(args.first().map(String::as_str), Some("flux"));
            assert!(args.contains(&"bootstrap".to_string()));
        } else {
            panic!("expected Shell kind");
        }
    }

    #[test]
    fn reconcile_plan_targets_flux_system() {
        use crate::core::operation::OperationKind;
        let mut c = base_config();
        c.namespace_exists = true;
        let plan = build_plan(&c).expect("plan");
        assert_eq!(plan.len(), 3);
        assert!(plan.iter().any(|o| o.id == "flux_reconcile_source_git"));
        let src = plan.first().expect("src");
        if let OperationKind::Shell { args, .. } = &src.kind {
            assert!(args.contains(&FLUX_SYNC_NAME.to_string()));
        } else {
            panic!("expected Shell kind");
        }
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
        use crate::core::operation::OperationKind;
        let mut c = base_config();
        c.git_url = "git@gitlab.com:futharkd/cluster.git".to_string();
        let plan = build_plan(&c).expect("plan");
        let bootstrap = plan
            .iter()
            .find(|o| o.id == "flux_bootstrap_git")
            .expect("bootstrap");
        if let OperationKind::Shell { args, .. } = &bootstrap.kind {
            assert!(
                args.iter()
                    .any(|a| a == "--url=ssh://git@gitlab.com/futharkd/cluster.git")
            );
        } else {
            panic!("expected Shell kind");
        }
    }
}
