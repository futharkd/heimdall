use anyhow::Result;

use super::input::BootstrapK3sConfig;
use crate::cli::K3sRole;
use crate::core::operation::{OperationKind, PlannedOperation};

fn kubectl_verify_operation() -> PlannedOperation {
    PlannedOperation {
        id: "k3s_kubectl_get_nodes",
        description: "Verify cluster API (`k3s kubectl get nodes -o name`)".to_string(),
        kind: OperationKind::Shell {
            command: "k3s".to_string(),
            args: vec![
                "kubectl".to_string(),
                "get".to_string(),
                "nodes".to_string(),
                "-o".to_string(),
                "name".to_string(),
            ],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    }
}

pub fn build_plan(config: &BootstrapK3sConfig) -> Result<Vec<PlannedOperation>> {
    if config.skip_install {
        let mut ops = Vec::new();
        if !config.skip_start {
            ops.push(kubectl_verify_operation());
        }
        return Ok(ops);
    }

    let install_path = config
        .install_script_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("install script path is not valid UTF-8"))?
        .to_string();

    let download = PlannedOperation {
        id: "download_official_install_script",
        description: "Download official k3s install script from get.k3s.io".to_string(),
        kind: OperationKind::Shell {
            command: "curl".to_string(),
            args: vec![
                "-fsSL".to_string(),
                "-o".to_string(),
                install_path.clone(),
                "https://get.k3s.io".to_string(),
            ],
            env: vec![],
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    };

    let mut install_env: Vec<(String, String)> = Vec::new();
    if let Some(v) = &config.version {
        install_env.push(("INSTALL_K3S_VERSION".to_string(), v.clone()));
    }
    if let Some(exec) = &config.install_exec {
        install_env.push(("INSTALL_K3S_EXEC".to_string(), exec.clone()));
    }
    if config.skip_start {
        install_env.push(("INSTALL_K3S_SKIP_START".to_string(), "true".to_string()));
    }
    if config.skip_enable {
        install_env.push(("INSTALL_K3S_SKIP_ENABLE".to_string(), "true".to_string()));
    }
    if config.role == K3sRole::Agent {
        if let Some(url) = &config.server_url {
            install_env.push(("K3S_URL".to_string(), url.clone()));
        }
        if let Some(token) = &config.token {
            install_env.push(("K3S_TOKEN".to_string(), token.clone()));
        }
    }

    let run_install = PlannedOperation {
        id: "run_official_install_script",
        description:
            "Execute official k3s install script (delegates to systemd and upstream layout)"
                .to_string(),
        kind: OperationKind::Shell {
            command: "sh".to_string(),
            args: vec![install_path],
            env: install_env,
            stdin_input: None,
        },
        requires_confirmation: false,
        failure_is_warning: false,
        verify: None,
    };

    let mut ops = vec![download, run_install];

    if !config.skip_start {
        ops.push(kubectl_verify_operation());
    }

    Ok(ops)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::cli::K3sRole;

    use super::build_plan;
    use crate::features::bootstrap::k3s::input::BootstrapK3sConfig;

    fn server_config() -> BootstrapK3sConfig {
        BootstrapK3sConfig {
            install_script_path: PathBuf::from("/tmp/heimdall-k3s-test.sh"),
            role: K3sRole::Server,
            server_url: None,
            token: None,
            version: Some("v1.30.1+k3s1".to_string()),
            install_exec: Some("--disable traefik".to_string()),
            skip_start: false,
            skip_enable: false,
            force: false,
            skip_install: false,
            dry_run: false,
        }
    }

    fn agent_config() -> BootstrapK3sConfig {
        BootstrapK3sConfig {
            install_script_path: PathBuf::from("/tmp/heimdall-k3s-agent.sh"),
            role: K3sRole::Agent,
            server_url: Some("https://cp.example:6443".to_string()),
            token: Some("secret-agent-token".to_string()),
            version: None,
            install_exec: None,
            skip_start: false,
            skip_enable: false,
            force: false,
            skip_install: false,
            dry_run: false,
        }
    }

    #[test]
    fn plan_downloads_get_k3s_io_and_sets_version_exec() {
        use crate::core::operation::OperationKind;
        let plan = build_plan(&server_config()).expect("plan");
        let dl = plan.first().expect("download");
        assert_eq!(dl.id, "download_official_install_script");
        if let OperationKind::Shell { args, .. } = &dl.kind {
            assert!(args.contains(&"https://get.k3s.io".to_string()));
        } else {
            panic!("expected Shell kind");
        }

        let inst = plan.get(1).expect("install");
        assert_eq!(inst.id, "run_official_install_script");
        if let OperationKind::Shell { env, .. } = &inst.kind {
            assert!(
                env.iter()
                    .any(|(k, v)| k == "INSTALL_K3S_VERSION" && v == "v1.30.1+k3s1")
            );
            assert!(
                env.iter()
                    .any(|(k, v)| k == "INSTALL_K3S_EXEC" && v == "--disable traefik")
            );
            assert!(!env.iter().any(|(k, _)| k == "K3S_TOKEN"));
        } else {
            panic!("expected Shell kind");
        }
    }

    #[test]
    fn plan_agent_includes_k3s_url_and_token() {
        use crate::core::operation::OperationKind;
        let plan = build_plan(&agent_config()).expect("plan");
        let inst = plan.get(1).expect("install");
        if let OperationKind::Shell { env, .. } = &inst.kind {
            assert!(
                env.iter()
                    .any(|(k, v)| k == "K3S_URL" && v == "https://cp.example:6443")
            );
            assert!(
                env.iter()
                    .any(|(k, v)| k == "K3S_TOKEN" && v == "secret-agent-token")
            );
        } else {
            panic!("expected Shell kind");
        }
    }

    #[test]
    fn plan_skip_start_omits_kubectl_verify() {
        let mut c = server_config();
        c.skip_start = true;
        let plan = build_plan(&c).expect("plan");
        assert_eq!(plan.len(), 2);
        assert!(!plan.iter().any(|o| o.id == "k3s_kubectl_get_nodes"));
    }

    #[test]
    fn plan_sets_skip_flags_in_env() {
        use crate::core::operation::OperationKind;
        let mut c = server_config();
        c.skip_start = true;
        c.skip_enable = true;
        let plan = build_plan(&c).expect("plan");
        let inst = plan.get(1).expect("install");
        if let OperationKind::Shell { env, .. } = &inst.kind {
            assert!(
                env.iter()
                    .any(|(k, v)| k == "INSTALL_K3S_SKIP_START" && v == "true")
            );
            assert!(
                env.iter()
                    .any(|(k, v)| k == "INSTALL_K3S_SKIP_ENABLE" && v == "true")
            );
        } else {
            panic!("expected Shell kind");
        }
    }

    #[test]
    fn plan_skip_install_omits_download_and_install_keeps_verify() {
        let mut c = server_config();
        c.skip_install = true;
        let plan = build_plan(&c).expect("plan");
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].id, "k3s_kubectl_get_nodes");
        assert!(
            !plan
                .iter()
                .any(|o| o.id == "download_official_install_script")
        );
    }

    #[test]
    fn plan_skip_install_and_skip_start_yields_empty_plan() {
        let mut c = server_config();
        c.skip_install = true;
        c.skip_start = true;
        let plan = build_plan(&c).expect("plan");
        assert!(plan.is_empty());
    }
}
