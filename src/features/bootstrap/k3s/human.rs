use crate::output::{Style, format_operation_report};

use super::report::BootstrapK3sReport;

pub fn format_report_human(report: &BootstrapK3sReport, style: &Style) -> String {
    format_operation_report("heimdall bootstrap k3s", &report.operations, style)
}
