use anyhow::{Result, bail};

pub fn validate_management_url(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        bail!("management URL must not be empty");
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        bail!("management URL must start with http:// or https://");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_management_url;

    #[test]
    fn rejects_non_http_management_url() {
        assert!(validate_management_url("ftp://x").is_err());
    }

    #[test]
    fn accepts_https_management_url() {
        assert!(validate_management_url("https://netbird.example:443").is_ok());
    }
}
