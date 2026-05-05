use crate::output::{StatusTone, Style};

use super::report::{CheckStatus, DoctorReport};

pub fn format_report_human(report: &DoctorReport, style: &Style) -> String {
    let mut lines = Vec::with_capacity(report.checks.len() + 4);
    lines.push(style.command_heading_block("heimdall doctor"));

    for check in &report.checks {
        let (label, tone) = match check.status {
            CheckStatus::Pass => ("PASS", StatusTone::Ok),
            CheckStatus::Warn => ("WARN", StatusTone::Warn),
            CheckStatus::Fail => ("FAIL", StatusTone::Fail),
        };
        let token = style.status_token(label, tone);
        lines.push(format!("{token}  {}", style.bold(check.description)));
        let detail = check.detail.trim();
        if !detail.is_empty() {
            for line in detail.lines() {
                lines.push(format!("         {}", style.dim(line)));
            }
        }
        lines.push(String::new());
    }

    lines.join("\n")
}
