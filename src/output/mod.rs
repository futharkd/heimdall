use crate::core::operation::OperationStatus;
use crate::features::bootstrap::netbird::report::BootstrapNetbirdReport;
use crate::features::bootstrap::user::report::BootstrapUserReport;
use crate::features::update::report::UpdateReport;
use crate::features::verify::doctor::report::{CheckStatus, DoctorReport};

pub fn render_doctor_human(report: &DoctorReport) -> String {
    let mut lines = Vec::with_capacity(report.checks.len() + 1);
    lines.push("heimdall doctor report".to_string());

    for check in &report.checks {
        let icon = match check.status {
            CheckStatus::Pass => "PASS",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
        };
        lines.push(format!(
            "- [{icon}] {}: {}",
            check.description, check.detail
        ));
    }

    lines.join("\n")
}

pub fn render_bootstrap_netbird_human(report: &BootstrapNetbirdReport) -> String {
    let mut lines = Vec::with_capacity(report.operations.len() + 1);
    lines.push("heimdall bootstrap netbird report".to_string());

    for operation in &report.operations {
        let state = match operation.status {
            OperationStatus::Planned => "PLAN",
            OperationStatus::Skipped => "SKIP",
            OperationStatus::Succeeded => "OK",
            OperationStatus::Failed => "FAIL",
        };
        lines.push(format!(
            "- [{state}] {}: {}",
            operation.description, operation.detail
        ));
    }

    lines.join("\n")
}

pub fn render_update_human(report: &UpdateReport) -> String {
    let mut lines = Vec::with_capacity(report.operations.len() + 6);
    lines.push("heimdall update report".to_string());
    lines.push(format!("- channel: {}", report.channel));
    lines.push(format!("- exe: {}", report.exe_path));
    lines.push(format!("- checksum url: {}", report.checksum_url));
    lines.push(format!("- binary url: {}", report.binary_url));
    if let (Some(local), Some(remote)) = (&report.local_digest, &report.remote_digest) {
        lines.push(format!("- local sha256:  {local}"));
        lines.push(format!("- remote sha256: {remote}"));
    }

    for operation in &report.operations {
        let state = match operation.status {
            OperationStatus::Planned => "PLAN",
            OperationStatus::Skipped => "SKIP",
            OperationStatus::Succeeded => "OK",
            OperationStatus::Failed => "FAIL",
        };
        lines.push(format!(
            "- [{state}] {}: {}",
            operation.description, operation.detail
        ));
    }

    lines.join("\n")
}

pub fn render_bootstrap_user_human(report: &BootstrapUserReport) -> String {
    let mut lines = Vec::with_capacity(report.operations.len() + 1);
    lines.push("heimdall bootstrap user report".to_string());

    for operation in &report.operations {
        let state = match operation.status {
            OperationStatus::Planned => "PLAN",
            OperationStatus::Skipped => "SKIP",
            OperationStatus::Succeeded => "OK",
            OperationStatus::Failed => "FAIL",
        };
        lines.push(format!(
            "- [{state}] {}: {}",
            operation.description, operation.detail
        ));
    }

    lines.join("\n")
}
