use crate::runner::{CommandRunner, LocalRunner};

use super::report::{CheckStatus, DoctorCheck, DoctorReport};

pub fn run() -> DoctorReport {
    let checks = vec![cargo_available(), cwd_readable(), git_repo_present()];
    DoctorReport { checks }
}

fn cargo_available() -> DoctorCheck {
    let runner = LocalRunner;
    match runner.run("cargo", &["--version"]) {
        Ok(output) if output.status.success() => DoctorCheck {
            id: "cargo_available",
            description: "Cargo executable is available",
            status: CheckStatus::Pass,
            detail: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        },
        Ok(output) => DoctorCheck {
            id: "cargo_available",
            description: "Cargo executable is available",
            status: CheckStatus::Fail,
            detail: format!("cargo returned non-zero status: {}", output.status),
        },
        Err(err) => DoctorCheck {
            id: "cargo_available",
            description: "Cargo executable is available",
            status: CheckStatus::Fail,
            detail: format!("unable to execute cargo: {err}"),
        },
    }
}

fn cwd_readable() -> DoctorCheck {
    match std::env::current_dir().and_then(std::fs::read_dir) {
        Ok(_) => DoctorCheck {
            id: "cwd_readable",
            description: "Current working directory is readable",
            status: CheckStatus::Pass,
            detail: "able to list current directory".to_string(),
        },
        Err(err) => DoctorCheck {
            id: "cwd_readable",
            description: "Current working directory is readable",
            status: CheckStatus::Fail,
            detail: format!("cannot read current directory: {err}"),
        },
    }
}

fn git_repo_present() -> DoctorCheck {
    if std::path::Path::new(".git").exists() {
        DoctorCheck {
            id: "git_repo_present",
            description: "Git repository metadata is present",
            status: CheckStatus::Pass,
            detail: ".git directory found".to_string(),
        }
    } else {
        DoctorCheck {
            id: "git_repo_present",
            description: "Git repository metadata is present",
            status: CheckStatus::Warn,
            detail: ".git directory not found; some commands may be limited".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::report::{CheckStatus, DoctorCheck, DoctorReport};

    #[test]
    fn report_detects_failures() {
        let report = DoctorReport {
            checks: vec![
                DoctorCheck {
                    id: "a",
                    description: "a",
                    status: CheckStatus::Pass,
                    detail: "ok".to_string(),
                },
                DoctorCheck {
                    id: "b",
                    description: "b",
                    status: CheckStatus::Fail,
                    detail: "no".to_string(),
                },
            ],
        };
        assert!(report.has_failures());
    }
}
