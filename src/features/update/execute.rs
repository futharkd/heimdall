use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use inquire::Confirm;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::core::operation::{OperationResult, OperationStatus};
use crate::runner::{CommandRunner, IoMode};

use super::checksum::{hex_lower, parse_sha256sum_file, sha256_bytes, sha256_file};
use super::input::UpdateConfig;
use super::report::UpdateReport;

fn map_inquire<T>(r: Result<T, inquire::InquireError>) -> anyhow::Result<T> {
    r.map_err(|e| match e {
        inquire::InquireError::NotTTY => anyhow::anyhow!("not a TTY; pass the flag directly"),
        inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted => {
            anyhow::anyhow!("cancelled")
        }
        other => anyhow::anyhow!("{other}"),
    })
}

pub fn execute_update(
    runner: &dyn CommandRunner,
    config: &UpdateConfig,
    io_mode: IoMode,
) -> UpdateReport {
    match run_update(runner, config, io_mode) {
        Ok(report) => report,
        Err(err) => UpdateReport {
            channel: config.package_version.clone(),
            binary_url: config.binary_url.clone(),
            checksum_url: config.checksum_url.clone(),
            exe_path: config.exe_path.display().to_string(),
            local_digest: None,
            remote_digest: None,
            operations: vec![OperationResult {
                id: "update",
                description: "Run heimdall update",
                status: OperationStatus::Failed,
                detail: err.to_string(),
            }],
        },
    }
}

fn run_update(
    runner: &dyn CommandRunner,
    config: &UpdateConfig,
    io_mode: IoMode,
) -> Result<UpdateReport> {
    let mut operations = Vec::new();

    let workdir = config
        .exe_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .with_context(|| format!("no parent directory for {}", config.exe_path.display()))?;

    let pid = std::process::id();
    let tmp_checksum = workdir.join(format!(".heimdall-update.{pid}.sha256"));
    let tmp_binary = workdir.join(format!(".heimdall-update.{pid}.new"));

    let curl_args = curl_download_args(
        &config.checksum_url,
        &tmp_checksum,
        config.github_token.as_deref(),
    );
    let outcome = run_curl(runner, &curl_args, io_mode);
    match outcome {
        Ok(()) => {
            operations.push(OperationResult {
                id: "fetch_remote_checksum",
                description: "Download published .sha256 for the selected package channel",
                status: OperationStatus::Succeeded,
                detail: format_curl_command("curl", &curl_args),
            });
        }
        Err(detail) => {
            operations.push(OperationResult {
                id: "fetch_remote_checksum",
                description: "Download published .sha256 for the selected package channel",
                status: OperationStatus::Failed,
                detail,
            });
            return Ok(finish_report(config, operations, None, None));
        }
    }

    let checksum_text = fs::read_to_string(&tmp_checksum)
        .with_context(|| format!("read {}", tmp_checksum.display()))?;
    let _ = fs::remove_file(&tmp_checksum);

    let remote_digest = parse_sha256sum_file(&checksum_text)?;
    let remote_hex = hex_lower(&remote_digest);

    let local_digest = sha256_file(&config.exe_path)?;
    let local_hex = hex_lower(&local_digest);

    let digests_match = remote_digest == local_digest;
    let needs_binary = config.force || !digests_match;

    operations.push(OperationResult {
        id: "evaluate_update",
        description: "Compare remote digest to the running binary",
        status: OperationStatus::Succeeded,
        detail: evaluate_detail(digests_match, config.force, &local_hex, &remote_hex),
    });

    if !needs_binary {
        operations.push(OperationResult {
            id: "download_binary",
            description: "Download published Linux amd64 binary",
            status: OperationStatus::Skipped,
            detail: "already up to date (remote digest matches running binary)".to_string(),
        });
        operations.push(OperationResult {
            id: "verify_download",
            description: "Verify downloaded bytes against the published digest",
            status: OperationStatus::Skipped,
            detail: "skipped (no download)".to_string(),
        });
        operations.push(OperationResult {
            id: "replace_binary",
            description: "Atomically replace the running binary",
            status: OperationStatus::Skipped,
            detail: "skipped (no download)".to_string(),
        });
        return Ok(finish_report(
            config,
            operations,
            Some(local_hex),
            Some(remote_hex),
        ));
    }

    if config.dry_run {
        let binary_args = curl_download_args(
            &config.binary_url,
            &tmp_binary,
            config.github_token.as_deref(),
        );
        operations.push(OperationResult {
            id: "download_binary",
            description: "Download published Linux amd64 binary",
            status: OperationStatus::Planned,
            detail: format!("dry-run: {}", format_curl_command("curl", &binary_args)),
        });
        operations.push(OperationResult {
            id: "verify_download",
            description: "Verify downloaded bytes against the published digest",
            status: OperationStatus::Planned,
            detail: format!("dry-run: sha256(download) == {remote_hex}"),
        });
        operations.push(OperationResult {
            id: "replace_binary",
            description: "Atomically replace the running binary",
            status: OperationStatus::Planned,
            detail: format!(
                "dry-run: rename {} -> {}",
                tmp_binary.display(),
                config.exe_path.display()
            ),
        });
        return Ok(finish_report(
            config,
            operations,
            Some(local_hex),
            Some(remote_hex),
        ));
    }

    if !config.yes {
        let approved = confirm_replace(&config.exe_path)?;
        if !approved {
            operations.push(OperationResult {
                id: "confirm_replace",
                description: "Confirm replacing the running binary",
                status: OperationStatus::Failed,
                detail: "aborted: not confirmed".to_string(),
            });
            return Ok(finish_report(
                config,
                operations,
                Some(local_hex),
                Some(remote_hex),
            ));
        }
        operations.push(OperationResult {
            id: "confirm_replace",
            description: "Confirm replacing the running binary",
            status: OperationStatus::Succeeded,
            detail: "confirmed".to_string(),
        });
    }

    let _ = fs::remove_file(&tmp_binary);
    let binary_args = curl_download_args(
        &config.binary_url,
        &tmp_binary,
        config.github_token.as_deref(),
    );
    match run_curl(runner, &binary_args, io_mode) {
        Ok(()) => {
            operations.push(OperationResult {
                id: "download_binary",
                description: "Download published Linux amd64 binary",
                status: OperationStatus::Succeeded,
                detail: format_curl_command("curl", &binary_args),
            });
        }
        Err(detail) => {
            operations.push(OperationResult {
                id: "download_binary",
                description: "Download published Linux amd64 binary",
                status: OperationStatus::Failed,
                detail,
            });
            let _ = fs::remove_file(&tmp_binary);
            return Ok(finish_report(
                config,
                operations,
                Some(local_hex),
                Some(remote_hex),
            ));
        }
    }

    let downloaded =
        fs::read(&tmp_binary).with_context(|| format!("read {}", tmp_binary.display()))?;
    let downloaded_digest = sha256_bytes(&downloaded);
    if downloaded_digest != remote_digest {
        let _ = fs::remove_file(&tmp_binary);
        operations.push(OperationResult {
            id: "verify_download",
            description: "Verify downloaded bytes against the published digest",
            status: OperationStatus::Failed,
            detail: format!(
                "checksum mismatch: expected {remote_hex}, got {}",
                hex_lower(&downloaded_digest)
            ),
        });
        return Ok(finish_report(
            config,
            operations,
            Some(local_hex),
            Some(remote_hex),
        ));
    }

    operations.push(OperationResult {
        id: "verify_download",
        description: "Verify downloaded bytes against the published digest",
        status: OperationStatus::Succeeded,
        detail: format!("sha256 matches remote digest ({remote_hex})"),
    });

    apply_executable_permissions(&config.exe_path, &tmp_binary)?;

    match fs::rename(&tmp_binary, &config.exe_path) {
        Ok(()) => {
            operations.push(OperationResult {
                id: "replace_binary",
                description: "Atomically replace the running binary",
                status: OperationStatus::Succeeded,
                detail: format!("replaced {}", config.exe_path.display()),
            });
        }
        Err(err) => {
            let _ = fs::remove_file(&tmp_binary);
            operations.push(OperationResult {
                id: "replace_binary",
                description: "Atomically replace the running binary",
                status: OperationStatus::Failed,
                detail: format!(
                    "rename failed (try sudo or install to a writable location): {err} (target {})",
                    config.exe_path.display()
                ),
            });
        }
    }

    Ok(finish_report(
        config,
        operations,
        Some(local_hex),
        Some(remote_hex),
    ))
}

fn finish_report(
    config: &UpdateConfig,
    operations: Vec<OperationResult>,
    local_digest: Option<String>,
    remote_digest: Option<String>,
) -> UpdateReport {
    UpdateReport {
        channel: config.package_version.clone(),
        binary_url: config.binary_url.clone(),
        checksum_url: config.checksum_url.clone(),
        exe_path: config.exe_path.display().to_string(),
        local_digest,
        remote_digest,
        operations,
    }
}

fn evaluate_detail(digests_match: bool, force: bool, local_hex: &str, remote_hex: &str) -> String {
    if digests_match && force {
        format!("remote {remote_hex} matches local {local_hex}; continuing because --force was set")
    } else if digests_match {
        format!("remote {remote_hex} matches local {local_hex}")
    } else {
        format!("remote {remote_hex} differs from local {local_hex}; update required")
    }
}

fn curl_download_args(url: &str, dest: &Path, token: Option<&str>) -> Vec<String> {
    let mut args = vec!["-fSL".to_string()];
    if let Some(token) = token {
        args.push("-H".to_string());
        args.push(format!("Authorization: token {token}"));
    }
    args.push("-o".to_string());
    args.push(dest.to_string_lossy().into_owned());
    args.push(url.to_string());
    args
}

fn run_curl(runner: &dyn CommandRunner, args: &[String], io_mode: IoMode) -> Result<(), String> {
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    match runner.run_with_env_io("curl", &arg_refs, &[], io_mode) {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(format!(
            "curl exit {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Err(err) => Err(format!("failed to execute curl: {err}")),
    }
}

fn format_curl_command(program: &str, args: &[String]) -> String {
    let display_args = redact_curl_headers(args);
    let joined = display_args.join(" ");
    format!("{program} {joined}")
}

fn redact_curl_headers(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] == "-H" && index + 1 < args.len() {
            out.push(args[index].clone());
            let value = &args[index + 1];
            if value.starts_with("Authorization:") {
                out.push("Authorization: <redacted>".to_string());
            } else {
                out.push(value.clone());
            }
            index += 2;
        } else {
            out.push(args[index].clone());
            index += 1;
        }
    }
    out
}

fn confirm_replace(path: &Path) -> Result<bool> {
    map_inquire(
        Confirm::new(&format!(
            "Replace running heimdall binary at {}?",
            path.display()
        ))
        .with_default(false)
        .prompt(),
    )
}

fn apply_executable_permissions(source_exe: &Path, downloaded: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let old_mode = fs::metadata(source_exe)
            .with_context(|| format!("stat {}", source_exe.display()))?
            .permissions()
            .mode();
        let mut perms = fs::metadata(downloaded)
            .with_context(|| format!("stat {}", downloaded.display()))?
            .permissions();
        perms.set_mode(old_mode | 0o111);
        fs::set_permissions(downloaded, perms)
            .with_context(|| format!("chmod {}", downloaded.display()))?;
        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (source_exe, downloaded);
        anyhow::bail!("unsupported platform");
    }
}

#[cfg(test)]
fn success_exit_status() -> std::process::ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatusExt::from_raw(0)
    }
    #[cfg(not(unix))]
    {
        std::process::Command::new("cmd")
            .args(["/C", "exit", "0"])
            .status()
            .expect("spawn cmd")
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs;
    use std::path::PathBuf;
    use std::rc::Rc;

    use super::{
        apply_executable_permissions, execute_update, format_curl_command, redact_curl_headers,
        success_exit_status,
    };
    use crate::cli::OutputFormat;
    use crate::core::operation::OperationStatus;
    use crate::features::update::checksum::{hex_lower, sha256_bytes};
    use crate::features::update::input::UpdateConfig;
    use crate::runner::{CommandRunner, IoMode};

    #[test]
    fn redact_curl_headers_masks_authorization() {
        let args = vec![
            "-fSL".to_string(),
            "-H".to_string(),
            "Authorization: token supersecret".to_string(),
            "-o".to_string(),
            "/tmp/x".to_string(),
            "https://example.com".to_string(),
        ];
        let redacted = redact_curl_headers(&args);
        let joined = redacted.join(" ");
        assert!(joined.contains("Authorization: <redacted>"));
        assert!(!joined.contains("supersecret"));
    }

    #[test]
    fn format_curl_command_redacts_when_requested() {
        let args = vec![
            "-fSL".to_string(),
            "-H".to_string(),
            "Authorization: token abc".to_string(),
            "-o".to_string(),
            "/tmp/x".to_string(),
            "https://example.com".to_string(),
        ];
        let text = format_curl_command("curl", &args);
        assert!(!text.contains("abc"));
    }

    struct RecordingRunner {
        pub calls: Rc<RefCell<Vec<Vec<String>>>>,
        pub sha256_body: String,
        pub binary_body: Vec<u8>,
    }

    impl CommandRunner for RecordingRunner {
        fn run_with_env_io(
            &self,
            program: &str,
            args: &[&str],
            _env: &[(&str, &str)],
            _mode: IoMode,
        ) -> anyhow::Result<std::process::Output> {
            self.calls
                .borrow_mut()
                .push(args.iter().map(|value| (*value).to_string()).collect());

            if program == "curl" {
                let mut output_path: Option<PathBuf> = None;
                let mut url: Option<String> = None;
                let mut index = 0;
                while index < args.len() {
                    if args[index] == "-o" && index + 1 < args.len() {
                        output_path = Some(PathBuf::from(args[index + 1]));
                        index += 2;
                        continue;
                    }
                    if args[index].starts_with("http://") || args[index].starts_with("https://") {
                        url = Some(args[index].to_string());
                        index += 1;
                        continue;
                    }
                    index += 1;
                }

                if let (Some(path), Some(url)) = (output_path, url) {
                    if url.ends_with(".sha256") {
                        fs::write(&path, self.sha256_body.as_bytes())?;
                    } else {
                        fs::write(&path, &self.binary_body)?;
                    }
                }
            }

            Ok(std::process::Output {
                status: success_exit_status(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }

        fn run_with_stdin(
            &self,
            _program: &str,
            _args: &[&str],
            _env: &[(&str, &str)],
            _stdin_data: &str,
            _mode: IoMode,
        ) -> anyhow::Result<std::process::Output> {
            Ok(std::process::Output {
                status: success_exit_status(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
    }

    fn sample_config(exe: PathBuf, force: bool, dry_run: bool, yes: bool) -> UpdateConfig {
        UpdateConfig {
            dry_run,
            yes,
            force,
            output: OutputFormat::Human,
            exe_path: exe,
            package_version: "latest".to_string(),
            binary_url: "https://github.com/futharkd/heimdall/releases/latest/download/heimdall-linux-amd64".to_string(),
            checksum_url: "https://github.com/futharkd/heimdall/releases/latest/download/heimdall-linux-amd64.sha256".to_string(),
            github_token: Some("secret-token".to_string()),
        }
    }

    #[test]
    fn skips_binary_when_up_to_date_without_force() {
        let dir = tempfile::tempdir().expect("tempdir");
        let exe = dir.path().join("heimdall");
        fs::write(&exe, b"same-bytes").expect("write exe");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&exe).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&exe, perms).unwrap();
        }

        let digest = sha256_bytes(b"same-bytes");
        let sha_body = format!("{}  heimdall-linux-amd64\n", hex_lower(&digest));

        let calls = Rc::new(RefCell::new(Vec::new()));
        let runner = RecordingRunner {
            calls: Rc::clone(&calls),
            sha256_body: sha_body,
            binary_body: b"ignored".to_vec(),
        };

        let report = execute_update(
            &runner,
            &sample_config(exe, false, false, true),
            IoMode::Buffered,
        );
        assert!(!report.has_failures());
        assert_eq!(calls.borrow().len(), 1);
        let details: String = report
            .operations
            .iter()
            .map(|op| op.detail.as_str())
            .collect();
        assert!(!details.contains("secret-token"));
    }

    #[test]
    fn force_triggers_second_curl_even_when_digest_matches() {
        let dir = tempfile::tempdir().expect("tempdir");
        let exe = dir.path().join("heimdall");
        fs::write(&exe, b"same-bytes").expect("write exe");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&exe).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&exe, perms).unwrap();
        }

        let digest = sha256_bytes(b"same-bytes");
        let sha_body = format!("{}  heimdall-linux-amd64\n", hex_lower(&digest));

        let calls = Rc::new(RefCell::new(Vec::new()));
        let runner = RecordingRunner {
            calls: Rc::clone(&calls),
            sha256_body: sha_body,
            binary_body: b"same-bytes".to_vec(),
        };

        let report = execute_update(
            &runner,
            &sample_config(exe.clone(), true, false, true),
            IoMode::Buffered,
        );
        assert!(!report.has_failures());
        assert_eq!(calls.borrow().len(), 2);

        let details: String = report
            .operations
            .iter()
            .map(|op| op.detail.as_str())
            .collect();
        assert!(!details.contains("secret-token"));
    }

    #[test]
    fn dry_run_with_force_plans_binary_without_second_curl() {
        let dir = tempfile::tempdir().expect("tempdir");
        let exe = dir.path().join("heimdall");
        fs::write(&exe, b"same-bytes").expect("write exe");

        let digest = sha256_bytes(b"same-bytes");
        let sha_body = format!("{}  heimdall-linux-amd64\n", hex_lower(&digest));

        let calls = Rc::new(RefCell::new(Vec::new()));
        let runner = RecordingRunner {
            calls: Rc::clone(&calls),
            sha256_body: sha_body,
            binary_body: b"nope".to_vec(),
        };

        let report = execute_update(
            &runner,
            &sample_config(exe, true, true, true),
            IoMode::Buffered,
        );
        assert!(!report.has_failures());
        assert_eq!(calls.borrow().len(), 1);
        let planned = report
            .operations
            .iter()
            .find(|operation| operation.id == "download_binary")
            .expect("download step");
        assert_eq!(planned.status, OperationStatus::Planned);
        assert!(planned.detail.contains("dry-run"));
        assert!(!planned.detail.contains("secret-token"));
    }

    #[test]
    fn apply_executable_permissions_sets_exec_bit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let exe = dir.path().join("heimdall");
        let new_bin = dir.path().join("newbin");
        fs::write(&exe, b"x").unwrap();
        fs::write(&new_bin, b"y").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&exe).unwrap().permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&exe, perms).unwrap();
            let mut perms_new = fs::metadata(&new_bin).unwrap().permissions();
            perms_new.set_mode(0o644);
            fs::set_permissions(&new_bin, perms_new).unwrap();

            apply_executable_permissions(&exe, &new_bin).expect("chmod");
            let mode = fs::metadata(&new_bin).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode & 0o111, 0o111);
        }
    }
}
