use crate::output::{Style, format_operation_report};

use super::report::ResetClusterReport;

pub fn format_report_human(report: &ResetClusterReport, style: &Style) -> String {
    format_operation_report("heimdall reset cluster", &report.operations, style)
}
