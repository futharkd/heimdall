use anyhow::{Result, bail};

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
    use super::validate_ssh_git_url;

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
}
