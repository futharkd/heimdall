pub fn validate_url(url: &str) -> anyhow::Result<()> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(anyhow::anyhow!(
            "install script URL must start with https:// or http://"
        ));
    }
    Ok(())
}

pub fn validate_username(username: &str) -> anyhow::Result<()> {
    if username.is_empty() {
        return Err(anyhow::anyhow!("username cannot be empty"));
    }
    if username.len() > 32 {
        return Err(anyhow::anyhow!("username too long (max 32 chars)"));
    }
    if !username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(anyhow::anyhow!(
            "username can only contain alphanumerics, underscore, and dash"
        ));
    }
    Ok(())
}

pub fn validate_registry_mirror(url: &str) -> anyhow::Result<()> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(anyhow::anyhow!(
            "registry mirror URL must start with https:// or http://"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_url_accepts_https() {
        assert!(validate_url("https://get.docker.com").is_ok());
    }

    #[test]
    fn validate_url_accepts_http() {
        assert!(validate_url("http://example.com/install.sh").is_ok());
    }

    #[test]
    fn validate_url_rejects_missing_scheme() {
        assert!(validate_url("get.docker.com").is_err());
    }

    #[test]
    fn validate_url_rejects_ftp() {
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn validate_username_accepts_alphanumeric() {
        assert!(validate_username("ubuntu").is_ok());
        assert!(validate_username("user123").is_ok());
    }

    #[test]
    fn validate_username_accepts_underscore_and_dash() {
        assert!(validate_username("my_user").is_ok());
        assert!(validate_username("my-user").is_ok());
    }

    #[test]
    fn validate_username_rejects_empty() {
        assert!(validate_username("").is_err());
    }

    #[test]
    fn validate_username_rejects_too_long() {
        assert!(validate_username(&"a".repeat(33)).is_err());
    }

    #[test]
    fn validate_username_rejects_special_chars() {
        assert!(validate_username("user@host").is_err());
        assert!(validate_username("user.name").is_err());
    }

    #[test]
    fn validate_registry_mirror_accepts_https() {
        assert!(validate_registry_mirror("https://mirror.example.com").is_ok());
    }

    #[test]
    fn validate_registry_mirror_rejects_missing_scheme() {
        assert!(validate_registry_mirror("mirror.example.com").is_err());
    }
}
