use crate::output::{Style, format_operation_report};

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

    lines.push(format_operation_report(
        "operations",
        &report.operations,
        style,
    ));

    lines.join("\n")
}
