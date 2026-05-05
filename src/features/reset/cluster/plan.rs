use anyhow::Result;

use crate::core::operation::{OperationKind, PlannedOperation};

use super::input::ResetClusterConfig;

pub fn build_plan(_config: &ResetClusterConfig) -> Result<Vec<PlannedOperation>> {
    Ok(vec![
        PlannedOperation {
            id: "flux_uninstall",
            description: "Uninstall Flux controllers and CRDs (best-effort)".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec![
                    "flux".to_string(),
                    "uninstall".to_string(),
                    "--silent".to_string(),
                    "--namespace".to_string(),
                    "flux-system".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: true,
            verify: None,
        },
        PlannedOperation {
            id: "flux_namespace_delete",
            description: "Delete Flux namespace (best-effort)".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec![
                    "kubectl".to_string(),
                    "delete".to_string(),
                    "namespace".to_string(),
                    "flux-system".to_string(),
                    "--ignore-not-found=true".to_string(),
                    "--wait=false".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: true,
            verify: None,
        },
        PlannedOperation {
            id: "k3s_killall",
            description: "Run k3s killall helper when present".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "if [ -x /usr/local/bin/k3s-killall.sh ]; then /usr/local/bin/k3s-killall.sh; elif [ -x /usr/local/bin/k3s-agent-killall.sh ]; then /usr/local/bin/k3s-agent-killall.sh; else echo 'note: no k3s killall helper found'; fi".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: true,
            verify: None,
        },
        PlannedOperation {
            id: "k3s_uninstall",
            description: "Run k3s uninstall script (server or agent)".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "if [ -x /usr/local/bin/k3s-uninstall.sh ]; then /usr/local/bin/k3s-uninstall.sh; elif [ -x /usr/local/bin/k3s-agent-uninstall.sh ]; then /usr/local/bin/k3s-agent-uninstall.sh; else echo 'k3s uninstall script not found' >&2; exit 1; fi".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: false,
            verify: None,
        },
        PlannedOperation {
            id: "remove_k3s_and_cni_state",
            description: "Remove k3s and CNI state directories".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec![
                    "rm".to_string(),
                    "-rf".to_string(),
                    "/etc/rancher".to_string(),
                    "/var/lib/rancher/k3s".to_string(),
                    "/var/lib/kubelet".to_string(),
                    "/var/lib/cni".to_string(),
                    "/etc/cni/net.d".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: true,
            verify: None,
        },
        PlannedOperation {
            id: "remove_k3s_systemd_units",
            description: "Remove stale k3s systemd unit files".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec![
                    "rm".to_string(),
                    "-f".to_string(),
                    "/etc/systemd/system/k3s.service".to_string(),
                    "/etc/systemd/system/k3s-agent.service".to_string(),
                    "/etc/systemd/system/multi-user.target.wants/k3s.service".to_string(),
                    "/etc/systemd/system/multi-user.target.wants/k3s-agent.service".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: true,
            verify: None,
        },
        PlannedOperation {
            id: "remove_cni_links",
            description: "Remove common CNI bridge links (best-effort)".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "ip link delete cni0 >/dev/null 2>&1 || true; ip link delete flannel.1 >/dev/null 2>&1 || true".to_string(),
                ],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: true,
            verify: None,
        },
        PlannedOperation {
            id: "systemd_daemon_reload",
            description: "Reload systemd units after cleanup".to_string(),
            kind: OperationKind::Shell {
                command: "sudo".to_string(),
                args: vec!["systemctl".to_string(), "daemon-reload".to_string()],
                env: vec![],
                stdin_input: None,
            },
            requires_confirmation: false,
            failure_is_warning: true,
            verify: None,
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::build_plan;
    use crate::core::operation::OperationKind;
    use crate::features::reset::cluster::input::ResetClusterConfig;

    #[test]
    fn plan_contains_expected_order_and_sudo() {
        let cfg = ResetClusterConfig { dry_run: false };
        let plan = build_plan(&cfg).expect("plan");
        assert_eq!(plan.first().map(|o| o.id), Some("flux_uninstall"));
        assert_eq!(plan.get(3).map(|o| o.id), Some("k3s_uninstall"));
        assert_eq!(plan.last().map(|o| o.id), Some("systemd_daemon_reload"));
        assert!(plan.iter().all(|o| matches!(
            &o.kind,
            OperationKind::Shell { command, .. } if command == "sudo"
        )));
    }
}
