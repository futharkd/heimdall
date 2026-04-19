use anyhow::Result;

use crate::cli::NetbirdInstallMethod;

use super::input::BootstrapNetbirdConfig;

#[derive(Debug, Clone)]
pub struct NetbirdPlannedOperation {
    pub id: &'static str,
    pub description: &'static str,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub failure_is_warning: bool,
}

pub fn build_plan(config: &BootstrapNetbirdConfig) -> Result<Vec<NetbirdPlannedOperation>> {
    let install_path = config
        .install_script_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("install script path is not valid UTF-8"))?
        .to_string();

    let download = NetbirdPlannedOperation {
        id: "download_official_install_script",
        description: "Download official NetBird install.sh from pkgs.netbird.io",
        command: "curl".to_string(),
        args: vec![
            "-fsSL".to_string(),
            "-o".to_string(),
            install_path.clone(),
            "https://pkgs.netbird.io/install.sh".to_string(),
        ],
        env: vec![],
        failure_is_warning: false,
    };

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

    let run_install = NetbirdPlannedOperation {
        id: "run_official_install_script",
        description: "Execute official NetBird install.sh (delegates to apt/dnf/yum or binary upstream)",
        command: "sh".to_string(),
        args: vec![install_path],
        env: install_env,
        failure_is_warning: false,
    };

    let mut up_args = vec!["up".to_string()];
    if let Some(key) = &config.setup_key {
        up_args.push("--setup-key".to_string());
        up_args.push(key.clone());
    }
    if let Some(url) = &config.management_url {
        up_args.push("--management-url".to_string());
        up_args.push(url.clone());
    }

    let netbird_up = NetbirdPlannedOperation {
        id: "netbird_up",
        description: "Join NetBird network (official `netbird up` CLI)",
        command: "netbird".to_string(),
        args: up_args,
        env: vec![],
        failure_is_warning: false,
    };

    let status = NetbirdPlannedOperation {
        id: "netbird_status",
        description: "Verify NetBird client status",
        command: "netbird".to_string(),
        args: vec!["status".to_string()],
        env: vec![],
        failure_is_warning: false,
    };

    let wt0 = NetbirdPlannedOperation {
        id: "verify_wt0_interface",
        description: "Check for wt0 interface (optional; may be absent until fully connected)",
        command: "ip".to_string(),
        args: vec!["link".to_string(), "show".to_string(), "wt0".to_string()],
        env: vec![],
        failure_is_warning: true,
    };

    Ok(vec![download, run_install, netbird_up, status, wt0])
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::cli::NetbirdInstallMethod;

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
        assert!(
            dl.args
                .contains(&"https://pkgs.netbird.io/install.sh".to_string())
        );

        let inst = plan.get(1).expect("install step");
        assert_eq!(inst.id, "run_official_install_script");
        assert!(
            inst.env
                .iter()
                .any(|(k, v)| k == "NETBIRD_RELEASE" && v == "latest")
        );
        assert!(
            inst.env
                .iter()
                .any(|(k, v)| k == "SKIP_UI_APP" && v == "true")
        );
        assert!(
            inst.env
                .iter()
                .any(|(k, v)| k == "USE_BIN_INSTALL" && v == "true")
        );
    }

    #[test]
    fn plan_package_install_sets_debian_frontend_not_use_bin() {
        let plan = build_plan(&sample_config_package()).expect("plan");
        let inst = plan.get(1).expect("install step");
        assert!(
            inst.env
                .iter()
                .any(|(k, v)| k == "DEBIAN_FRONTEND" && v == "noninteractive")
        );
        assert!(!inst.env.iter().any(|(k, _)| k == "USE_BIN_INSTALL"));
    }

    #[test]
    fn plan_netbird_up_includes_setup_key_and_management_url() {
        let plan = build_plan(&sample_config()).expect("plan");
        let up = plan.iter().find(|s| s.id == "netbird_up").expect("up");
        let pos = up
            .args
            .iter()
            .position(|a| a == "--setup-key")
            .expect("--setup-key");
        assert!(!up.args[pos + 1].is_empty());
        let mpos = up
            .args
            .iter()
            .position(|a| a == "--management-url")
            .expect("--management-url");
        assert_eq!(up.args[mpos + 1], "https://netbird.example:443");
    }
}
