use crate::output::{Style, format_operation_report};

use super::report::BootstrapUserReport;

pub fn format_report_human(report: &BootstrapUserReport, style: &Style) -> String {
    format_operation_report("heimdall bootstrap user", &report.operations, style)
}
