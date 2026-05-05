use crate::core::doctor::DoctorContext;
use crate::features::bootstrap::flux::input::{
    kubeconfig_requires_elevated_access, probe_flux_namespace,
};

use super::super::report::{CheckStatus, DoctorCheck};

const FLUX_NAMESPACE_DEFAULT: &str = "flux-system";

fn default_kubeconfig() -> String {
    std::env::var("KUBECONFIG")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/etc/rancher/k3s/k3s.yaml".to_string())
}

pub fn contribute(ctx: &DoctorContext) -> Vec<DoctorCheck> {
    let kubeconfig = default_kubeconfig();
    let elevated = kubeconfig_requires_elevated_access(&kubeconfig);
    match probe_flux_namespace(ctx.runner, &kubeconfig, FLUX_NAMESPACE_DEFAULT, elevated) {
        Ok(true) => vec![DoctorCheck {
            id: "bootstrap_flux",
            description: "Flux namespace present (flux-system)",
            status: CheckStatus::Pass,
            detail: format!(
                "kubectl get ns {} succeeded (KUBECONFIG={})",
                FLUX_NAMESPACE_DEFAULT, kubeconfig
            ),
        }],
        Ok(false) => vec![DoctorCheck {
            id: "bootstrap_flux",
            description: "Flux namespace present (flux-system)",
            status: CheckStatus::Warn,
            detail: format!(
                "namespace {} not found or API unreachable (KUBECONFIG={})",
                FLUX_NAMESPACE_DEFAULT, kubeconfig
            ),
        }],
        Err(e) => vec![DoctorCheck {
            id: "bootstrap_flux",
            description: "Flux namespace present (flux-system)",
            status: CheckStatus::Warn,
            detail: format!("probe failed: {e:#}"),
        }],
    }
}
