use anyhow::{Result, bail};

pub fn validate_username(username: &str) -> Result<()> {
    if username.is_empty() {
        bail!("username must not be empty");
    }

    let first = username.as_bytes()[0] as char;
    if !(first.is_ascii_lowercase() || first == '_') {
        bail!("username must start with a lowercase letter or underscore");
    }

    if !username
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        bail!("username contains unsupported characters");
    }

    Ok(())
}

pub fn validate_ssh_key(key: &str) -> Result<()> {
    let mut parts = key.split_whitespace();
    let algo = parts.next().unwrap_or_default();
    let payload = parts.next().unwrap_or_default();
    if algo.is_empty() || payload.is_empty() {
        bail!("ssh key must contain algorithm and payload");
    }

    let supported = ["ssh-ed25519", "ssh-rsa", "ecdsa-sha2-nistp256"];
    if !supported.contains(&algo) {
        bail!("unsupported ssh key algorithm: {algo}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_ssh_key;

    #[test]
    fn rejects_invalid_key() {
        assert!(validate_ssh_key("not-a-key").is_err());
    }
}
