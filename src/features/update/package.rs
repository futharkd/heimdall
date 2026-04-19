use anyhow::{Context, Result, bail};

const DEFAULT_PACKAGE_NAME: &str = "heimdall";
const BINARY_NAME: &str = "heimdall-linux-amd64";

/// Returns `(api_origin, encoded_project_path)` for GitLab Generic Package download URLs.
pub fn parse_gitlab_repository(repository: &str) -> Result<(String, String)> {
    let trimmed = repository.trim().trim_end_matches('/');
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .with_context(|| format!("repository URL must be https:// or http://: {repository}"))?;

    let (host, path) = rest
        .split_once('/')
        .with_context(|| format!("repository URL missing project path: {repository}"))?;

    if host.is_empty() || path.is_empty() {
        bail!("invalid repository URL: {repository}");
    }

    let project_path = path.trim_end_matches(".git");
    let encoded = url_encode_project_path(project_path);

    Ok((format!("https://{host}"), encoded))
}

fn url_encode_project_path(project_path: &str) -> String {
    project_path
        .chars()
        .map(|ch| match ch {
            '/' => "%2F".to_string(),
            ch if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') => ch.to_string(),
            ch => {
                let mut encoded = String::new();
                for byte in ch.to_string().as_bytes() {
                    encoded.push_str(&format!("%{byte:02X}"));
                }
                encoded
            }
        })
        .collect()
}

pub fn generic_artifact_url(
    api_origin: &str,
    encoded_project: &str,
    package_name: &str,
    package_version: &str,
    artifact: &str,
) -> String {
    format!(
        "{api_origin}/api/v4/projects/{encoded_project}/packages/generic/{package_name}/{package_version}/{artifact}"
    )
}

pub fn binary_and_checksum_urls(
    api_origin: &str,
    encoded_project: &str,
    package_version: &str,
) -> (String, String) {
    let binary = generic_artifact_url(
        api_origin,
        encoded_project,
        DEFAULT_PACKAGE_NAME,
        package_version,
        BINARY_NAME,
    );
    let checksum = generic_artifact_url(
        api_origin,
        encoded_project,
        DEFAULT_PACKAGE_NAME,
        package_version,
        &format!("{BINARY_NAME}.sha256"),
    );
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
    use super::{
        binary_and_checksum_urls, generic_artifact_url, parse_gitlab_repository,
        validate_package_version,
    };

    #[test]
    fn parses_default_repository() {
        let (origin, encoded) =
            parse_gitlab_repository("https://gitlab.com/futharkd/heimdall").expect("parse");
        assert_eq!(origin, "https://gitlab.com");
        assert_eq!(encoded, "futharkd%2Fheimdall");
    }

    #[test]
    fn strips_git_suffix() {
        let (_, encoded) =
            parse_gitlab_repository("https://gitlab.com/futharkd/heimdall.git").expect("parse");
        assert_eq!(encoded, "futharkd%2Fheimdall");
    }

    #[test]
    fn builds_latest_urls() {
        let (bin, sha) =
            binary_and_checksum_urls("https://gitlab.com", "futharkd%2Fheimdall", "latest");
        assert_eq!(
            bin,
            "https://gitlab.com/api/v4/projects/futharkd%2Fheimdall/packages/generic/heimdall/latest/heimdall-linux-amd64"
        );
        assert_eq!(
            sha,
            "https://gitlab.com/api/v4/projects/futharkd%2Fheimdall/packages/generic/heimdall/latest/heimdall-linux-amd64.sha256"
        );
    }

    #[test]
    fn builds_tagged_urls() {
        let (bin, _) =
            binary_and_checksum_urls("https://gitlab.com", "futharkd%2Fheimdall", "v0.1.0");
        assert!(bin.contains("/heimdall/v0.1.0/heimdall-linux-amd64"));
    }

    #[test]
    fn generic_url_encodes_version_segment_via_caller() {
        let url = generic_artifact_url(
            "https://example.com",
            "group%2Fproj",
            "heimdall",
            "latest",
            "heimdall-linux-amd64",
        );
        assert!(url.contains("/packages/generic/heimdall/latest/"));
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
