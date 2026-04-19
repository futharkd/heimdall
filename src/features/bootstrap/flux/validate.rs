use anyhow::{Result, bail};

/// Converts SCP-style `user@host:org/repo.git` to `ssh://user@host/org/repo.git`.
///
/// Flux parses `--url` with Go's URL parser, which rejects the colon in `host:path`. Normalizing
/// avoids `parse "git@…": first path segment in URL cannot contain colon`.
pub fn normalize_ssh_git_url_for_flux(url: &str) -> String {
    let t = url.trim();
    if t.starts_with("ssh://") {
        return t.to_string();
    }
    if let Some(at) = t.find('@') {
        let after_at = &t[at + 1..];
        if let Some(colon) = after_at.find(':') {
            let host = &after_at[..colon];
            let path = &after_at[colon + 1..];
            if !host.is_empty() && !path.is_empty() {
                let user = &t[..at];
                return format!("ssh://{user}@{host}/{path}");
            }
        }
    }
    t.to_string()
}

pub fn finalize_flux_git_url(url: &str) -> Result<String> {
    validate_ssh_git_url(url)?;
    Ok(normalize_ssh_git_url_for_flux(url))
}

pub fn validate_ssh_git_url(url: &str) -> Result<()> {
    let t = url.trim();
    if t.is_empty() {
        bail!("Git URL must not be empty");
    }
    if t.starts_with("https://") || t.starts_with("http://") {
        bail!(
            "use an SSH Git URL (https:// is for PAT + --token-auth; not supported in this bootstrap)"
        );
    }
    if t.starts_with("ssh://") {
        return Ok(());
    }
    if t.starts_with("git@") && t.contains(':') {
        return Ok(());
    }
    bail!("Git URL must start with ssh:// or use scp form git@host:path");
}

#[cfg(test)]
mod tests {
    use super::{finalize_flux_git_url, normalize_ssh_git_url_for_flux, validate_ssh_git_url};

    #[test]
    fn rejects_https() {
        assert!(validate_ssh_git_url("https://gitlab.com/a/b.git").is_err());
    }

    #[test]
    fn accepts_ssh_scheme() {
        assert!(validate_ssh_git_url("ssh://git@gitlab.com/group/repo.git").is_ok());
    }

    #[test]
    fn accepts_git_at_scp() {
        assert!(validate_ssh_git_url("git@github.com:org/repo.git").is_ok());
    }

    #[test]
    fn normalize_scp_to_ssh_scheme() {
        assert_eq!(
            normalize_ssh_git_url_for_flux("git@gitlab.com:futharkd/cluster.git"),
            "ssh://git@gitlab.com/futharkd/cluster.git"
        );
    }

    #[test]
    fn normalize_leaves_ssh_scheme_unchanged() {
        let u = "ssh://git@gitlab.com/group/repo.git";
        assert_eq!(normalize_ssh_git_url_for_flux(u), u);
    }

    #[test]
    fn finalize_normalizes_scp() {
        assert_eq!(
            finalize_flux_git_url("git@gitlab.com:futharkd/cluster.git").unwrap(),
            "ssh://git@gitlab.com/futharkd/cluster.git"
        );
    }
}
