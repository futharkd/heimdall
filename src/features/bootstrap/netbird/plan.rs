use anyhow::Result;

use crate::cli::NetbirdInstallMethod;
use crate::core::operation::{OperationKind, PlannedOperation};

use super::input::BootstrapNetbirdConfig;

pub fn build_plan(config: &BootstrapNetbirdConfig) -> Result<Vec<PlannedOperation>> {
    let install_path = config
        .install_script_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("install script path is not valid UTF-8"))?
        .to_string();

    let mut ops = vec![PlannedOperation {
        id: "download_official_install_script",
        description: "Download official NetBird install.sh from pkgs.netbird.io".to_string(),
        kind: OperationKind::Shell {
            command: "curl".to_string(),
            args: vec![
                "-fsSL".to_string(),
                "-o".to_string(),
                install_path.clone(),
                "https://pkgs.netbird.io/install.sh".to_string(),
            ],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    }];

    let mut install_env: Vec<(String, String)> =
        vec![("NETBIRD_RELEASE".to_string(), config.release.clone())];
    if config.skip_ui {
        install_env.push(("SKIP_UI_APP".to_string(), "true".to_string()));
    }
    if let Some(token) = &config.github_token {
        install_env.push(("GITHUB_TOKEN".to_string(), token.clone()));
    }
    match config.install_method {
        NetbirdInstallMethod::Binary => {
            install_env.push(("USE_BIN_INSTALL".to_string(), "true".to_string()));
        }
        NetbirdInstallMethod::Package => {
            install_env.push(("DEBIAN_FRONTEND".to_string(), "noninteractive".to_string()));
        }
    }

    ops.push(PlannedOperation {
        id: "run_official_install_script",
        description: "Execute official NetBird install.sh (delegates to apt/dnf/yum or binary upstream)".to_string(),
        kind: OperationKind::Shell {
            command: "sh".to_string(),
            args: vec![install_path],
            env: install_env,
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    let mut up_args = vec!["up".to_string()];
    if let Some(key) = &config.setup_key {
        up_args.push("--setup-key".to_string());
        up_args.push(key.clone());
    }
    if let Some(url) = &config.management_url {
        up_args.push("--management-url".to_string());
        up_args.push(url.clone());
    }

    ops.push(PlannedOperation {
        id: "netbird_up",
        description: "Join NetBird network (official `netbird up` CLI)".to_string(),
        kind: OperationKind::Shell {
            command: "netbird".to_string(),
            args: up_args,
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    ops.push(PlannedOperation {
        id: "netbird_status",
        description: "Verify NetBird client status".to_string(),
        kind: OperationKind::Shell {
            command: "netbird".to_string(),
            args: vec!["status".to_string()],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    });

    ops.push(PlannedOperation {
        id: "verify_wt0_interface",
        description: "Check for wt0 interface (optional; may be absent until fully connected)".to_string(),
        kind: OperationKind::Shell {
            command: "ip".to_string(),
            args: vec!["link".to_string(), "show".to_string(), "wt0".to_string()],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: true,
        verify: None,
    });

    Ok(ops)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::cli::NetbirdInstallMethod;
    use crate::core::operation::OperationKind;

    use super::build_plan;
    use crate::features::bootstrap::netbird::input::BootstrapNetbirdConfig;

    fn sample_config() -> BootstrapNetbirdConfig {
        BootstrapNetbirdConfig {
            install_script_path: PathBuf::from("/tmp/heimdall-netbird-test.sh"),
            skip_ui: true,
            release: "latest".to_string(),
            install_method: NetbirdInstallMethod::Binary,
            github_token: None,
            setup_key: Some("secret-setup-key".to_string()),
            management_url: Some("https://netbird.example:443".to_string()),
            dry_run: false,
        }
    }

    fn sample_config_package() -> BootstrapNetbirdConfig {
        BootstrapNetbirdConfig {
            install_script_path: PathBuf::from("/tmp/heimdall-netbird-test.sh"),
            skip_ui: true,
            release: "latest".to_string(),
            install_method: NetbirdInstallMethod::Package,
            github_token: None,
            setup_key: Some("secret-setup-key".to_string()),
            management_url: Some("https://netbird.example:443".to_string()),
            dry_run: false,
        }
    }

    #[test]
    fn plan_downloads_official_script_and_runs_install_with_release_env() {
        let plan = build_plan(&sample_config()).expect("plan");
        let dl = plan.first().expect("download step");
        assert_eq!(dl.id, "download_official_install_script");
        if let OperationKind::Shell { args, .. } = &dl.kind {
            assert!(args.contains(&"https://pkgs.netbird.io/install.sh".to_string()));
        } else {
            panic!("expected Shell kind");
        }

        let inst = plan.get(1).expect("install step");
        assert_eq!(inst.id, "run_official_install_script");
        if let OperationKind::Shell { env, .. } = &inst.kind {
            assert!(env.iter().any(|(k, v)| k == "NETBIRD_RELEASE" && v == "latest"));
            assert!(env.iter().any(|(k, v)| k == "SKIP_UI_APP" && v == "true"));
            assert!(env.iter().any(|(k, v)| k == "USE_BIN_INSTALL" && v == "true"));
        } else {
            panic!("expected Shell kind");
        }
    }

    #[test]
    fn plan_package_install_sets_debian_frontend_not_use_bin() {
        let plan = build_plan(&sample_config_package()).expect("plan");
        let inst = plan.get(1).expect("install step");
        if let OperationKind::Shell { env, .. } = &inst.kind {
            assert!(env.iter().any(|(k, v)| k == "DEBIAN_FRONTEND" && v == "noninteractive"));
            assert!(!env.iter().any(|(k, _)| k == "USE_BIN_INSTALL"));
        } else {
            panic!("expected Shell kind");
        }
    }

    #[test]
    fn plan_netbird_up_includes_setup_key_and_management_url() {
        let plan = build_plan(&sample_config()).expect("plan");
        let up = plan.iter().find(|s| s.id == "netbird_up").expect("up");
        if let OperationKind::Shell { args, .. } = &up.kind {
            let pos = args.iter().position(|a| a == "--setup-key").expect("--setup-key");
            assert!(!args[pos + 1].is_empty());
            let mpos = args.iter().position(|a| a == "--management-url").expect("--management-url");
            assert_eq!(args[mpos + 1], "https://netbird.example:443");
        } else {
            panic!("expected Shell kind");
        }
    }
}
