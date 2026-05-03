use super::report::HardenFirewallReport;
use crate::output::{Style, StatusTone};

pub fn format_report_human(report: &HardenFirewallReport, style: &Style) -> String {
    let mut lines = Vec::new();

    lines.push(style.cyan("heimdall harden firewall").to_string());
    let dashes = "─".repeat(40);
    lines.push(style.dim(&dashes).to_string());

    for op in &report.operations {
        let (token_label, tone) = match op.status.as_str() {
            "planned" => ("PLAN", StatusTone::Planned),
            "succeeded" => ("OK", StatusTone::Ok),
            "skipped" => ("SKIP", StatusTone::Skip),
            "failed" => ("FAIL", StatusTone::Fail),
            _ => ("?", StatusTone::Skip),
        };

        let token = style.status_token(token_label, tone);
        let desc = style.bold(&op.description);
        lines.push(format!("{} {}", token, desc));

        if !op.detail.is_empty() {
            for detail_line in op.detail.lines() {
                lines.push(format!("  {}", style.dim(detail_line)));
            }
        }
    }

    lines.join("\n")
}
