use crate::core::operation::OperationStatus;
use crate::output::{StatusTone, Style};

use super::report::UpdateReport;

pub fn format_report_human(report: &UpdateReport, style: &Style) -> String {
    let mut lines = Vec::with_capacity(report.operations.len() + 10);
    lines.push(String::new());
    lines.push(style.cyan("heimdall update"));
    let rule = "─".repeat(48);
    lines.push(style.dim(&rule));
    lines.push(String::new());
    lines.push(format!(
        "{} {}",
        style.dim("channel:"),
        style.bold(&report.channel)
    ));
    lines.push(format!("{} {}", style.dim("exe:"), report.exe_path));
    lines.push(format!(
        "{} {}",
        style.dim("checksum:"),
        report.checksum_url
    ));
    lines.push(format!("{} {}", style.dim("binary:"), report.binary_url));
    if let (Some(local), Some(remote)) = (&report.local_digest, &report.remote_digest) {
        lines.push(format!("{} {}", style.dim("local sha256:"), local));
        lines.push(format!("{} {}", style.dim("remote sha256:"), remote));
    }
    lines.push(String::new());

    for operation in &report.operations {
        let (label, tone) = match operation.status {
            OperationStatus::Planned => ("PLAN", StatusTone::Planned),
            OperationStatus::Skipped => ("SKIP", StatusTone::Skip),
            OperationStatus::Succeeded => ("OK", StatusTone::Ok),
            OperationStatus::Failed => ("FAIL", StatusTone::Fail),
        };
        let token = style.status_token(label, tone);
        lines.push(format!(
            "{token}  {}",
            style.bold(operation.description.as_str())
        ));
        let detail = operation.detail.trim();
        if !detail.is_empty() {
            for line in detail.lines() {
                lines.push(format!("         {}", style.dim(line)));
            }
        }
        lines.push(String::new());
    }

    lines.join("\n")
}
