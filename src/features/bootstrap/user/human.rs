use crate::core::operation::OperationStatus;
use crate::output::{StatusTone, Style};

use super::report::BootstrapUserReport;

pub fn format_report_human(report: &BootstrapUserReport, style: &Style) -> String {
    let mut lines = Vec::with_capacity(report.operations.len() + 4);
    lines.push(String::new());
    lines.push(style.cyan("heimdall bootstrap user"));
    let rule = "─".repeat(48);
    lines.push(style.dim(&rule));
    lines.push(String::new());

    for operation in &report.operations {
        let (label, tone) = match operation.status {
            OperationStatus::Planned => ("PLAN", StatusTone::Planned),
            OperationStatus::Skipped => ("SKIP", StatusTone::Skip),
            OperationStatus::Succeeded => ("OK", StatusTone::Ok),
            OperationStatus::Failed => ("FAIL", StatusTone::Fail),
        };
        let token = style.status_token(label, tone);
        lines.push(format!("{token}  {}", style.bold(operation.description)));
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
