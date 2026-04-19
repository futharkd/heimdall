use anyhow::{Result, bail};

use crate::cli::K3sRole;

pub fn validate_k3s_server_url(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        bail!("K3S server URL must not be empty");
    }
    if !trimmed.starts_with("https://") {
        bail!("K3S server URL must start with https://");
    }
    let rest = trimmed.strip_prefix("https://").expect("prefix checked");
    let host_port = rest.split('/').next().unwrap_or("");
    if host_port.is_empty() {
        bail!("K3S server URL must include a host");
    }
    Ok(())
}

pub fn validate_agent_inputs(
    role: K3sRole,
    server_url: Option<&str>,
    token: Option<&str>,
) -> Result<()> {
    if role != K3sRole::Agent {
        return Ok(());
    }
    let Some(url) = server_url.filter(|s| !s.trim().is_empty()) else {
        bail!("--role agent requires --server-url or K3S_URL");
    };
    validate_k3s_server_url(url)?;
    if token.map(str::trim).is_none_or(|t| t.is_empty()) {
        bail!("--role agent requires --token or K3S_TOKEN");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_agent_inputs, validate_k3s_server_url};
    use crate::cli::K3sRole;

    #[test]
    fn rejects_http_url() {
        assert!(validate_k3s_server_url("http://x:6443").is_err());
    }

    #[test]
    fn accepts_https_with_host() {
        assert!(validate_k3s_server_url("https://server:6443").is_ok());
    }

    #[test]
    fn agent_requires_url_and_token() {
        assert!(validate_agent_inputs(K3sRole::Agent, None, Some("t")).is_err());
        assert!(validate_agent_inputs(K3sRole::Agent, Some("https://h:6443"), None).is_err());
        assert!(validate_agent_inputs(K3sRole::Agent, Some("https://h:6443"), Some("tok")).is_ok());
    }

    #[test]
    fn server_skips_agent_validation() {
        assert!(validate_agent_inputs(K3sRole::Server, None, None).is_ok());
    }
}
