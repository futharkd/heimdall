use anyhow::{Context, Result, bail};

const BINARY_NAME: &str = "heimdall-linux-amd64";

/// Returns `(owner, repo)` from a GitHub repository URL.
pub fn parse_github_repository(repository: &str) -> Result<(String, String)> {
    let trimmed = repository.trim().trim_end_matches('/');
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .with_context(|| format!("repository URL must be https:// or http://: {repository}"))?;

    let (_host, path) = rest
        .split_once('/')
        .with_context(|| format!("repository URL missing project path: {repository}"))?;

    let path = path.trim_end_matches(".git");
    let (owner, repo) = path
        .split_once('/')
        .with_context(|| format!("repository URL missing repo name: {repository}"))?;

    Ok((owner.to_string(), repo.to_string()))
}

pub fn release_artifact_url(owner: &str, repo: &str, version: &str, artifact: &str) -> String {
    if version == "latest" {
        format!("https://github.com/{owner}/{repo}/releases/latest/download/{artifact}")
    } else {
        format!("https://github.com/{owner}/{repo}/releases/download/{version}/{artifact}")
    }
}

pub fn binary_and_checksum_urls(owner: &str, repo: &str, version: &str) -> (String, String) {
    let binary = release_artifact_url(owner, repo, version, BINARY_NAME);
    let checksum = release_artifact_url(owner, repo, version, &format!("{BINARY_NAME}.sha256"));
    (binary, checksum)
}

pub fn validate_package_version(version: &str) -> Result<()> {
    if version.is_empty() {
        bail!("package version must not be empty");
    }
    if !version
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+' | '~'))
    {
        bail!(
            "package version contains unsupported characters (use letters, digits, and ._-+~ only): {version}"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{binary_and_checksum_urls, parse_github_repository, validate_package_version};

    #[test]
    fn parses_default_repository() {
        let (owner, repo) =
            parse_github_repository("https://github.com/futharkd/heimdall").expect("parse");
        assert_eq!(owner, "futharkd");
        assert_eq!(repo, "heimdall");
    }

    #[test]
    fn strips_git_suffix() {
        let (owner, repo) =
            parse_github_repository("https://github.com/futharkd/heimdall.git").expect("parse");
        assert_eq!(owner, "futharkd");
        assert_eq!(repo, "heimdall");
    }

    #[test]
    fn builds_latest_urls() {
        let (bin, sha) = binary_and_checksum_urls("futharkd", "heimdall", "latest");
        assert_eq!(
            bin,
            "https://github.com/futharkd/heimdall/releases/latest/download/heimdall-linux-amd64"
        );
        assert_eq!(
            sha,
            "https://github.com/futharkd/heimdall/releases/latest/download/heimdall-linux-amd64.sha256"
        );
    }

    #[test]
    fn builds_tagged_urls() {
        let (bin, _) = binary_and_checksum_urls("futharkd", "heimdall", "v0.1.0");
        assert!(bin.contains("/releases/download/v0.1.0/heimdall-linux-amd64"));
    }

    #[test]
    fn validate_package_version_accepts_common_tags() {
        validate_package_version("latest").expect("latest");
        validate_package_version("v1.2.3-rc.1").expect("semver-ish");
    }

    #[test]
    fn validate_package_version_rejects_slash() {
        assert!(validate_package_version("bad/tag").is_err());
    }
}
